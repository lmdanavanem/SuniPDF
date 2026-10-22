# Bibliothèques d’annotations

[English](ANNOTATION_LIBRARY.md) · [Périmètre et qualification 0.45](RELEASE_0.45.0.fr.md)

## Dans l’application

Ouvrez **Bibliothèque d’annotations** depuis la barre principale, **Tous les outils** ou **Commenter**. Son fichier local est distinct des préférences de l’application.

1. Choisissez **Créer un modèle** pour préparer une nouvelle annotation. La capture de brouillon prend en charge texte en fonte standard, polygones/polylignes, nuages de révision, tampons prédéfinis et tampons image PNG.
2. Dans **Enregistrer dans la bibliothèque**, donnez un nom unique au modèle puis enregistrez-le. Auteur, source et licence sont des déclarations facultatives ; une information absente reste inconnue. Le modèle n’est pas appliqué au PDF à cette étape.
3. Dans la bibliothèque, choisissez **Placer**, réglez l’échelle puis cliquez sur la page pour positionner le modèle. Vérifiez **Aperçu**, puis **Appliquer**. Annuler abandonne le placement ; l’application constitue une seule étape Annuler.
4. Pour créer un groupe, sélectionnez au moins deux entrées, choisissez **Grouper**, réglez leurs décalages puis enregistrez un nouveau nom. Un groupe plat contient au plus 16 primitives. Chaque élément reçoit une nouvelle identité d’annotation lors du placement.

L’import fusionne une bibliothèque validée avec la bibliothèque active. Identifiants ou noms dupliqués et entrées invalides refusent l’import complet, sans remplacement partiel. L’export écrit toutes les entrées ou la sélection dans un nouveau fichier JSON. La suppression explicite retire les modèles sélectionnés ; elle ne retire pas les annotations déjà placées dans les PDF. Recharger relit le fichier local enregistré.

Les formes ordinaires sont également importables dans le format versionné. La capture arbitraire d’une annotation PDF sélectionnée, de son fil de revue ou de son apparence importée reste hors de ce premier lot. Aucune calibration de mesure n’est réutilisée implicitement par la bibliothèque.

## Exemple CLI

Enregistrez cette entrée dans `draft-entry.json` :

```json
{
  "id": "review-draft",
  "name": "Review draft",
  "provenance": { "author": null, "source": null, "license": null },
  "primitives": [
    { "kind": "stamp", "rect": [0, 0, 160, 50], "stamp": "draft" }
  ]
}
```

```powershell
sunipdf-cli annotation-library add --entry draft-entry.json --out review-library.json
sunipdf-cli annotation-library inspect review-library.json
sunipdf-cli annotation-library place review-library.json --id review-draft --file plan.pdf --page 1 --origin 40,40 --scale 1 --out plan-reviewed.pdf
sunipdf-cli annotation-library export review-library.json --id review-draft --out selected-library.json
sunipdf-cli annotation-library remove review-library.json --id review-draft --out remaining-library.json
```

Les pages commencent à 1. L’origine correspond au coin inférieur gauche de la page affichée, en points PDF, avec prise en compte de la rotation et du rognage. L’échelle est explicite et uniforme. Après transformation, géométrie et styles doivent toujours respecter la page et les limites des annotations. Une destination existante est refusée, même avec `--overwrite` : choisissez un nouveau nom. Un PDF protégé utilise les options existantes `--password-stdin` ou `--password-env`. Le placement exige le droit d’annotation et conserve le chiffrement pris en charge.

## Format ouvert et limites

L’enveloppe est `{"format":"sunipdf-annotation-library","version":1,"entries":[...]}`. Chaque entrée contient un identifiant ASCII, un nom unique, une provenance déclarée et une liste plate de primitives. Les types sont `shape`, `free_text`, `vertices`, `stamp` et `image`, avec géométrie et styles explicites. Une image contient ses octets PNG en base64, ses dimensions et son SHA256, sans conserver de chemin externe.

Limites : 16 Mio par fichier, 64 entrées, 16 primitives par entrée. Chaque PNG est limité à 4 Mio et quatre millions de pixels ; des budgets globaux d’images s’appliquent aussi. Noms et identifiants sont uniques. Versions, champs ou actions inconnus, images malformées et géométries invalides sont refusés. Le placement prend en charge les documents jusqu’à 128 Mio. Le refus d’un groupe ne laisse aucune insertion partielle.

Les données sont locales et autonomes. La provenance est conservée telle que déclarée ; elle ne prouve pas la propriété et ne constitue pas une vérification indépendante de licence. La bibliothèque n’importe ni scripts, pièces jointes, échelles de mesure, identités d’objets PDF, réponses ni actions exécutables. Après placement, les éléments restent des annotations PDF modifiables individuellement ; groupes imbriqués et capture générale d’annotations restent à développer.
