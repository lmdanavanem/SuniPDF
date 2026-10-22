# Tables de quantités

[English](QUANTITY_TABLE.md) · [Périmètre et qualification 0.46](RELEASE_0.46.0.fr.md) · [Mesures](MEASUREMENTS.fr.md) · [Registre d’annotations](MARKUP_REGISTER.fr.md)

L’implémentation 0.46 est en développement et en cours de validation. Ce guide décrit les contrôles et données pris en charge ; il n’établit ni publication ni qualification achevée.

## Ouvrir et filtrer la table

Ouvrez un PDF puis choisissez **Table des quantités** dans **Tous les outils**, dans les contrôles de mesure ou depuis le registre d’annotations. Terminez ou annulez d’abord tout autre brouillon actif. La consultation conserve le PDF intact.

Les lignes affichent page physique, indice d’annotation, type, contenu, quantité, unité, prix unitaire, coût de ligne et colonnes personnalisées disponibles. Pages en base 1 ; indices d’annotation en base 0. Un marqueur explicite compte pour `1 each`. Longueur, polyligne, périmètre ou surface enregistrés fournissent leur valeur mesurée et leur unité. Un angle est affiché en `deg`, sans addition ni tarification. Aucune quantité n’est déduite d’une annotation ordinaire.

Filtrez par recherche textuelle, type d’annotation, discipline, statut métier, groupe de comptage, auteur ou page physique. Dans le filtre de l’application, la page `0` désigne toutes les pages. **Groupes de sous-totaux** propose **Sans regroupement**, **Groupe de comptage**, **Discipline** et **Statut métier**. Les sous-totaux couvrent toutes les lignes correspondantes, au-delà de la page courante de 50 lignes. **50 précédentes** et **50 suivantes** changent de page de table ; **Filtrer les noms de colonnes** modifie les colonnes visibles sans changer les lignes ni les totaux. **Actualiser** relit le document après application ou annulation d’un brouillon.

## Modifier prix et champs personnalisés

1. Cliquez sur **Modifier la ligne** pour une annotation compatible. Son identité doit être non ambiguë, son état modifiable et le droit d’annotation disponible.
2. Renseignez **Prix unitaire**, un **Code devise** de trois lettres majuscules, **Prix par unité** et **Décimales du coût** (`0` à `4`). Utilisez le point décimal, sans expression. Un prix vide est absent ; `0` est un prix nul renseigné. L’unité de prix doit correspondre à la dimension de la quantité : une longueur en mètres peut être tarifée par pied, mais une surface ne peut pas être tarifée par mètre.
3. Dans **Colonnes personnalisées**, cliquez sur **Ajouter une colonne personnalisée** puis renseignez nom, type et valeur. Les types sont **Texte**, **Nombre**, **Date (AAAA-MM-JJ)** et **Choix**. Pour un choix, saisissez les options autorisées, une par ligne, puis l’une d’elles comme valeur. Un même nom de colonne doit garder le même type et les mêmes options partout dans le PDF. Les champs texte du registre existant restent distincts de ces champs typés.
4. Cliquez sur **Appliquer les champs** pour enregistrer la ligne, ou **Annuler la modification** pour abandonner le brouillon. **Retirer la colonne** retire sa valeur seulement lorsque la ligne est appliquée. Un texte vide reste renseigné ; retirer la colonne rend sa valeur absente. Valeurs invalides et cibles périmées conservent le PDF intact et le brouillon disponible pour correction ou annulation.

Chaque modification réussie constitue une étape Annuler. Fermez la table avant d’utiliser **Annuler** ou **Rétablir** dans l’application, puis rouvrez-la pour consulter les valeurs restaurées. Enregistrez le PDF pour conserver les champs après réouverture. Ces modifications ne changent ni sommets mesurés, ni calibration, ni identité d’annotation, ni commentaires liés. Aucun prix ne peut être attaché à une annotation sans quantité compatible ou à un angle ; des valeurs personnalisées peuvent être attachées aux annotations ordinaires modifiables.

## Quantités, coûts et valeurs absentes

**Non renseigné** désigne une donnée absente, jamais un zéro. La table ne remplace pas manuellement la quantité : modifiez la mesure ou les marqueurs dans leurs propres outils.

Les sous-totaux de quantités gardent leurs unités d’origine. `m` et `ft`, ou `m2` et `ft2`, restent séparés malgré la conversion possible vers une unité de prix compatible. Aucun total ne mélange longueur, surface et comptage. Unités : `each`, `mm`, `cm`, `m`, `in`, `ft`, `mm2`, `cm2`, `m2`, `in2`, `ft2` ; `deg` sert uniquement à l’affichage dans ce calcul.

Pour chaque ligne tarifée, la quantité est convertie vers l’unité explicite du prix puis quantifiée à six décimales. La multiplication par le prix décimal utilise ensuite des entiers avec contrôle de dépassement. Le coût de ligne est arrondi aux `0` à `4` décimales choisies, les demi-unités s’éloignant de zéro ; les sous-totaux additionnent ces coûts déjà arrondis. Les groupes monétaires restent séparés par code devise et précision. Un code devise est une déclaration, sans vérification d’un taux de change.

**Sous-totaux filtrés** signale les prix manquants et la couverture partielle des coûts. Un sous-total monétaire est complet seulement si toutes les lignes de son groupe de quantité contribuent à cette même devise et précision. Des devises mélangées peuvent donc produire des sous-totaux partiels même lorsque toutes les lignes ont un prix. Annotations sans quantité et angles sont exclus des totaux additifs et comptés dans `unquantified_rows`. L’écran présente au plus 30 groupes ; l’export conserve le récapitulatif filtré complet. Utilisez les filtres pour consulter d’autres groupes.

## Export et contrôles de source

Choisissez **Exporter CSV** ou **Exporter JSON** puis un nouveau nom de fichier. L’export contient toutes les lignes correspondantes, pas seulement la page ou les colonnes visibles. Le droit de copie est requis pour exporter un fichier ou du JSON CLI. La consultation à l’écran peut rester disponible lorsque l’export est interdit. Le droit d’annotation contrôle séparément les modifications.

L’export contient un `source_sha256` de l’instantané du document, les options de filtre/tri/regroupement, les lignes et les indicateurs de couverture. Il s’agit d’une trace de provenance, pas d’une certification de justesse physique ni d’une signature. Une modification ou un export préparé dans l’application est refusé si son instantané source a changé. La CLI exige une ligne actuelle exacte dans `--expected-record` ; après comparaison de cette ligne et de sa page, elle capture la source de l’opération. Cet argument n’est pas une option indépendante de vérification du hash attendu du fichier.

Le JSON utilise le schéma `sunipdf-quantity-table`, version `1`, avec `rows` et `summary`. `complete: true` signifie que la construction bornée de la table a réussi ; chaque total monétaire conserve son propre indicateur de couverture `complete`. Le CSV comprend les lignes `row`, `quantity_subtotal`, `cost_subtotal` et `metadata`, les données des colonnes typées et la neutralisation des formules de tableur dans les cellules texte. Conservez la ligne de métadonnées lors d’un traitement externe.

## CLI et JSON typé

Commandes : `quantity-table`, `set-quantity-fields` et `clear-quantity-fields`. Sans `--out`, le rapport renvoie du JSON sur stdout ; le CSV exige `--out`. Les rapports acceptent les filtres du registre, `--sort`, `--descending` et `--include-replies`, ainsi que `--group-by none|count_group|discipline|business_status`. La page CLI `--page` est une page physique strictement positive.

Enregistrez ce remplacement complet des champs dans `fields.json`, en UTF-8 :

```json
{
  "unit_price": {
    "amount": "12.50",
    "currency": "CHF",
    "per_unit": "each",
    "rounding_decimals": 2
  },
  "custom": {
    "Package": { "type": "text", "value": "Electrical" },
    "Allowance": { "type": "number", "value": "1.25" },
    "Due date": { "type": "date", "value": "2026-12-31" },
    "Stage": {
      "type": "choice",
      "value": "Reviewed",
      "options": ["Draft", "Reviewed"]
    }
  }
}
```

`amount` et les valeurs numériques typées sont des chaînes décimales. `per_unit` utilise les identifiants d’unités ci-dessus. `unit_price: null` efface le prix ; les clés personnalisées omises sont retirées, car `set-quantity-fields` remplace tous les champs de quantités de la ligne. `clear-quantity-fields` retire l’enregistrement complet de ces champs, sans retirer l’annotation ni ses données de mesure/comptage.

Cet exemple PowerShell ajoute un marqueur manuel dans un `plan.pdf` existant, extrait sa ligne exacte, applique les champs ci-dessus, exporte les coûts puis les efface avec une ligne attendue actualisée. Utilisez des noms de sortie nouveaux et arrêtez la séquence si une commande signale une erreur :

```powershell
$created = sunipdf-cli count plan.pdf --page 1 --group Devices --point "60,60" --out counted.pdf | ConvertFrom-Json
sunipdf-cli quantity-table counted.pdf --page 1 --group-by count_group --out quantities.json
$table = Get-Content -Raw quantities.json | ConvertFrom-Json
$row = $table.rows | Where-Object { $_.register.page -eq 1 -and $_.register.index -eq $created.index }
$row | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 expected-row.json
sunipdf-cli set-quantity-fields counted.pdf --page 1 --expected-record expected-row.json --fields fields.json --out priced.pdf
sunipdf-cli quantity-table priced.pdf --group-by count_group --format csv --out priced.csv
sunipdf-cli quantity-table priced.pdf --page 1 --out priced.json
$priced = Get-Content -Raw priced.json | ConvertFrom-Json
$updated = $priced.rows | Where-Object { $_.register.page -eq 1 -and $_.register.index -eq $created.index }
$updated | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 updated-row.json
sunipdf-cli clear-quantity-fields priced.pdf --page 1 --expected-record updated-row.json --out cleared.pdf
```

Toutes les modifications PDF et tous les exports explicites exigent une destination nouvelle, même avec `--overwrite`. Le chiffrement pris en charge est conservé dans les copies PDF. Utilisez `--password-env NOM` ou `--password-stdin` pour les entrées protégées. La sortie stdout d’une modification décrit l’opération demandée sans exporter le contenu source du PDF.

## Bornes et périmètre restant

Le registre est limité à 1 000 pages, des instantanés de 128 Mio, 10 000 entrées d’annotation et 4 096 entrées par page, avec des budgets de texte distincts. JSON de champs et fichiers de ligne attendue : 64 Kio maximum ; métadonnées de quantités : 4 Mio par document. Un document admet au plus 16 noms de colonnes typées. Noms : 64 octets UTF-8 maximum ; textes : 1 024 octets ; choix : 1 à 32 options distinctes non vides de 64 octets maximum. Dates invalides, doublons, types/options incohérents, versions de métadonnées incompatibles et identités ambiguës sont refusés.

Les entrées décimales admettent au plus six décimales et une valeur absolue de mille milliards ; les prix sont non négatifs. La quantité convertie pour tarification doit également être finie, non négative et au plus égale à mille milliards. Les valeurs excessives sont refusées, sans troncature silencieuse.

AEC-007 reste partiel : ni formules arbitraires, change monétaire, calcul fiscal, totaux entre documents, reconnaissance automatique de symboles ni processus complet de devis. Vérifiez séparément la révision et la calibration du plan avant d’utiliser ses quantités.
