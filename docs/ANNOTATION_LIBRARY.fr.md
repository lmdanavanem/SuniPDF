# Bibliothèques d’annotations

[English](ANNOTATION_LIBRARY.md) · [Périmètre et qualification 0.45](RELEASE_0.45.0.fr.md)

## Dans l’application

Cliquez sur **Bibliothèque** dans la barre principale, ou choisissez **Bibliothèque d’annotations** dans **Tous les outils** ou **Commenter**. Son fichier local est distinct des préférences de l’application. Sa gestion est possible sans PDF ouvert ; créer un brouillon ou placer un modèle exige un document ouvert.

Disponible dans [0.45.0](RELEASE_0.45.0.fr.md). [Preuves](qa/RELEASE_0.45.0.json).

1. Choisissez **Créer un modèle** pour préparer une nouvelle annotation. La capture prend en charge texte avec une police standard, polygones/polylignes, nuages de révision, tampons prédéfinis et tampons image PNG. Choisissez le type puis préparez texte, géométrie et style dans la boîte d’annotations enrichies.
2. Développez **Enregistrer dans la bibliothèque**, saisissez un **Nom** unique puis cliquez sur **Enregistrer le modèle**. **Crédit**, **Source ou provenance** et **Licence ou droit de réutilisation** sont des déclarations facultatives ; une valeur vide reste inconnue. L’enregistrement conserve le PDF intact et garde le brouillon ouvert. Fermez-le avec **Annuler** si vous souhaitez seulement garder le modèle.
3. Rouvrez la bibliothèque et choisissez **Placer sur la page**. Réglez **Échelle** dans la barre de placement, puis cliquez sur la page pour fixer le coin inférieur gauche du modèle. Un cadre suit le pointeur ; le modèle entier doit tenir dans la page. Le clic demande un aperçu rendu. Dans **Placer un modèle d’annotation**, vous pouvez ajuster **Origine X (pt)**, **Origine Y (pt)** et **Échelle** ; après un changement, redemandez **Aperçu**. Vérifiez le résultat puis cliquez sur **Appliquer à l’objet**. **Annuler** ou **Échap** abandonne le placement. Son application constitue une seule étape Annuler ; enregistrez le PDF pour la conserver.
4. Pour créer un groupe, cochez au moins deux entrées puis choisissez **Créer un groupe**. Saisissez un nouveau nom, réglez les décalages horizontal/vertical de chaque élément en points puis cliquez sur **Enregistrer le modèle**. **Annuler** abandonne le brouillon de groupe. Un groupe plat contient au plus 16 primitives ; les modèles sources restent disponibles. Chaque élément reçoit une nouvelle identité d’annotation au placement.

**Importer une bibliothèque** fusionne un fichier validé avec la bibliothèque active. Identifiants ou noms dupliqués, entrées invalides ou limites dépassées refusent l’import complet. **Exporter la bibliothèque** et **Exporter la sélection** écrivent un nouveau fichier JSON : choisissez un nom qui n’existe pas déjà. **Supprimer la sélection**, puis **Confirmer la suppression**, retire les modèles sélectionnés ; **Annuler** les conserve. Cette suppression ne retire pas les annotations déjà placées dans les PDF. **Filtrer les modèles** recherche leurs noms. **Recharger la bibliothèque** relit le fichier local.

Un échec de lecture, d’import ou d’enregistrement conserve la bibliothèque active. Si une autre instance a changé son fichier, rechargez-le avant d’enregistrer ; un enregistrement en cours doit finir avant qu’une autre instance puisse écrire. Annuler/Rétablir agit sur les annotations placées dans le PDF, pas sur le fichier de bibliothèque. Exportez une copie avant de supprimer des modèles à conserver.

L’échelle est un multiplicateur uniforme de taille : `1` conserve la taille du modèle. Elle est indépendante de la calibration des mesures. L’application ramène les limites locales du modèle à une origine inférieure gauche `(0, 0)` avant placement. Terminez ou annulez le placement avant de changer de document ou d’outil. Si son document change, sélectionnez de nouveau le modèle.

Les formes ordinaires sont également importables dans le format versionné. La capture arbitraire d’une annotation PDF sélectionnée, de son fil de revue ou de son apparence importée reste hors de ce premier lot. Mesures et échelles de page sont exclues.

## Exemple CLI

Enregistrez cette entrée dans un fichier JSON UTF-8 nommé `draft-entry.json` :

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
sunipdf-cli annotation-library place review-library.json --id review-draft --file plan.pdf --page 1 --origin "40,40" --scale 1 --out plan-reviewed.pdf
sunipdf-cli annotation-library export review-library.json --id review-draft --out selected-library.json
sunipdf-cli annotation-library remove review-library.json --id review-draft --out remaining-library.json
```

Les pages commencent à 1. L’origine utilise les coordonnées de la page affichée, en points PDF depuis son coin inférieur gauche, avec prise en compte de la rotation et du rognage. La CLI transforme chaque coordonnée locale selon `origine + coordonnée × échelle` ; une entrée écrite manuellement peut donc conserver une marge locale. L’échelle est explicite et uniforme. Après transformation, géométrie et styles doivent toujours respecter la page et les limites des annotations. Une destination existante est refusée, même avec `--overwrite` : choisissez un nouveau nom. Un PDF protégé utilise les options existantes `--password-stdin` ou `--password-env`. Le placement exige le droit d’annotation et conserve le chiffrement pris en charge.

## Format ouvert et limites

L’enveloppe est `{"format":"sunipdf-annotation-library","version":1,"entries":[...]}`. Chaque entrée contient un identifiant ASCII, un nom unique, une provenance déclarée et une liste plate de primitives. Les types sont `shape`, `free_text`, `vertices`, `stamp` et `image`, avec géométrie et styles explicites. Une image contient ses octets PNG en base64, ses dimensions et son SHA256, sans conserver de chemin externe.

Limites : 16 Mio par fichier, 64 entrées, 16 primitives par entrée. Chaque PNG est limité à 4 Mio, 4096 pixels par côté et quatre millions de pixels. Chaque entrée et la bibliothèque complète sont aussi limitées à 12 Mio de données d’images encodées et seize millions de pixels d’images. Noms et identifiants sont uniques. Versions, champs ou actions inconnus, images malformées et géométries invalides sont refusés. Le placement prend en charge les documents jusqu’à 128 Mio. Le refus d’un groupe ne laisse aucune insertion partielle.

Les données sont locales et autonomes. La provenance est conservée telle que déclarée ; elle ne prouve pas la propriété et ne constitue pas une vérification indépendante de licence. La bibliothèque n’importe ni scripts, pièces jointes, échelles de mesure, identités d’objets PDF, réponses ni actions exécutables. Après placement, les éléments restent des annotations PDF distinctes ; les contrôles de modification dépendent de leur type. Ils ne restent pas liés en groupe. Groupes imbriqués et capture générale d’annotations restent à développer.
