# Superposition de révisions

[English](REVISION_OVERLAY.md) · [Périmètre et qualification 0.47](RELEASE_0.47.0.fr.md)

Disponible dans [0.47.0](RELEASE_0.47.0.fr.md). [Preuves](qa/RELEASE_0.47.0.json).

## Interface

1. Ouvrez **Superposition de révisions** depuis **Tous les outils**, ou utilisez le bouton du même nom dans **Comparer des fichiers**. Celui-ci présélectionne les fichiers et pages de départ du comparateur. Aucun onglet PDF ouvert n’est requis.
2. Utilisez **Choisir le PDF de base** et **Choisir le PDF révisé**, puis réglez chaque **Page**. Il s’agit de numéros physiques à partir de 1. Chaque source possède son propre champ masqué **Mot de passe (facultatif)**. Choisissez des fichiers enregistrés : les modifications non enregistrées dans les onglets ne sont pas incluses.
3. Choisissez **Alignement manuel**, puis réglez **Décalage horizontal**, **Décalage vertical**, **Rotation horaire** et **Facteur d’échelle**. Lancez **Aperçu** pour charger les deux images sources et le résultat combiné.
4. Vous pouvez aussi choisir **Alignement par deux points**, puis **Aperçu** pour charger les sources. Cliquez le point 1 puis le point 2 sur la base, puis les points correspondants dans le même ordre sur la révision. Relancez **Aperçu** pour calculer l’alignement. Chaque paire doit contenir deux points distincts à l’intérieur de sa page. **Réinitialiser l’alignement** efface les quatre points et remet les décalages et la rotation à zéro, ainsi que l’échelle à 1.
5. Réglez **Opacité de la révision** et **Côté maximal de l’image** selon le besoin. La base reste magenta à pleine densité ; la révision est cyan. Relancez **Aperçu** après toute modification des réglages. L’export reste désactivé tant que l’aperçu affiché est périmé. Changer un fichier source ou une page efface aussi les images et l’alignement précédents.
6. Examinez **Aperçu de la superposition** et, au besoin, développez **Empreintes des sources (SHA-256)**. Utilisez **Exporter PNG** ou **Exporter le rapport JSON**, puis choisissez un nouveau nom. Chaque export écrit un seul fichier et vérifie à nouveau les deux sources enregistrées. Si l’une a changé, préparez un nouvel aperçu.

Passer d’un alignement par deux points calculé au mode manuel conserve sa transformation pour l’ajuster. Les points restent disponibles jusqu’à une réinitialisation ou un changement de fichier/page. Réinitialiser l’alignement conserve l’opacité et la résolution. Aucun réglage d’alignement n’est écrit dans les PDF.

**Annuler** ou **Échap** demande l’annulation pendant le traitement ; la boîte de dialogue attend la fin de l’opération bornée en cours. À l’arrêt, **Fermer** ou **Échap** la ferme. Un export déjà enregistré avant l’annulation reste présent et son résultat est signalé. Les mots de passe restent en mémoire et sont effacés à la fermeture ou au remplacement du fichier correspondant ; ils ne figurent pas dans le rapport.

## Coordonnées et limites d’image

Les coordonnées de superposition partent du **coin supérieur gauche de la page affichée**, après son recadrage et sa rotation. X augmente vers la droite et Y vers le bas. Un point PDF vaut 1/72 de pouce. Cette convention diffère des coordonnées depuis le coin inférieur gauche utilisées pour placer les annotations.

La transformation place la révision sur la base : échelle uniforme autour de l’origine supérieure gauche de la révision, rotation horaire autour de cette origine, puis décalages horizontal et vertical. Sans ajustement, l’échelle vaut 1, les décalages et la rotation zéro. Deux paires de points déterminent translation, rotation et échelle uniforme ; elles n’introduisent ni cisaillement ni correction de perspective.

La page de base fixe les dimensions et le cadre de sortie. Le contenu révisé qui dépasse est coupé ; le cadre ne s’agrandit pas pour l’inclure. Chaque source est rendue indépendamment dans la limite de pixels choisie, puis échantillonnée dans la superposition. Des pages de tailles physiques très différentes peuvent donc avoir des densités de pixels différentes. Agrandir une source ne restitue pas ses détails fins, et certaines différences de lignes peuvent provenir de la conversion en image.

| Réglage | Limite |
|---|---|
| Taille du PDF enregistré | 64 Mio par source |
| Pages choisies | Une page physique dans chaque PDF |
| Côté maximal de l’image | 1536 px par défaut ; interface : 512, 1024, 1536 ou 2048 ; CLI : 64–2048 |
| Opacité de la révision | 0–1 ; 0,65 par défaut |
| Échelle uniforme | 0,01–100 |
| Rotation horaire | −360° à 360° |
| Décalage horizontal/vertical | −1 000 000 à 1 000 000 pt |
| Paires de points | Deux points par page, distants d’au moins 0,001 pt, dans la page affichée |

Les deux sources doivent autoriser la copie, y compris avec un mot de passe propriétaire. Les exports PNG et JSON sont des fichiers ordinaires ; le chiffrement des PDF sources reste inchangé puisqu’aucune source n’est réécrite. Les empreintes identifient les octets exacts des fichiers, pas la précision de l’alignement ni l’exhaustivité d’une revue de révision.

## CLI

Ces exemples supposent que les pages choisies contiennent les points indiqués. Commencez par un alignement manuel :

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --base-page 1 --revision-page 1 --translate "10,-10" --rotation 0 --scale 1 --opacity 0.65 --max-side 1536 --out overlay-manual.png
```

Ou alignez deux paires correspondantes et écrivez le rapport :

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --base-points "50,50 150,50" --revision-points "40,60 140,60" --format json --out overlay-report.json
```

Les deux options de points sont requises ensemble et ne peuvent pas être combinées avec `--translate`, `--rotation` ou `--scale`. `--format` vaut `png` par défaut ; précisez `json` pour un rapport. Les destinations existantes sont refusées, même avec `--overwrite`. Chaque invocation crée un seul nouveau fichier. Les nombres décimaux de la CLI utilisent un point.

Pour imposer qu’un export PNG ultérieur utilise exactement les mêmes sources que le rapport :

```powershell
$report = Get-Content -LiteralPath overlay-report.json -Raw | ConvertFrom-Json
sunipdf-cli revision-overlay base.pdf revision.pdf --base-points "50,50 150,50" --revision-points "40,60 140,60" --expected-base-sha256 $report.base.source_sha256 --expected-revision-sha256 $report.revision.source_sha256 --out overlay-aligned.png
```

Pour des fichiers protégés, fournissez les mots de passe séparément. Avec deux variables d’environnement déjà définies :

```powershell
sunipdf-cli revision-overlay base.pdf revision.pdf --password-env SUNIPDF_BASE_PASSWORD --revision-password-env SUNIPDF_REVISION_PASSWORD --out overlay-protected-source.png
```

`--password-stdin` lit le mot de passe de la base et `--revision-password-stdin` celui de la révision. Un seul mot de passe peut utiliser stdin dans la même invocation ; utilisez une variable d’environnement pour l’autre. Les mots de passe ne sont pas automatiquement partagés entre les sources.

## Rapport JSON

Le rapport utilise `schema: "sunipdf-revision-overlay"` et `version: 1`. Ses objets `base` et `revision` consignent les chemins canoniques des sources, SHA-256, numéro physique de page, nombre de pages, dimensions affichées en points, boîte de recadrage effective en coordonnées utilisateur PDF, rotation de page et dimensions des images sources.

`transform` contient la translation, la rotation horaire et l’échelle uniforme calculées. `alignment` conserve le mode choisi et ses paramètres. Voici, par exemple, la section `alignment` de la commande manuelle ci-dessus ; il ne s’agit pas d’un rapport complet :

```json
{
  "mode": "manual",
  "transform": {
    "translation": [10, -10],
    "rotation_degrees": 0,
    "scale": 1
  }
}
```

Pour l’alignement par deux points, `alignment.mode` vaut `two_points` et `alignment.base` ainsi que `alignment.revision` contiennent les deux paires. Le rapport consigne aussi le cadre de base et le découpage, les couleurs, l’opacité, la limite de pixels, l’échantillonnage bilinéaire et les limites du rendu. Il ne contient aucune modification du PDF. L’ouverture d’un rapport pour restaurer les réglages de l’interface n’est pas incluse dans ce lot ; la CLI accepte des paramètres explicites et des empreintes sources attendues facultatives.

AEC-013 reste partiel. Le recalage automatique, les contours automatiques de changements et le suivi des révisions d’AEC-014 restent ouverts. Cet outil ne compare pas les vecteurs, n’effectue pas d’OCR, ne fusionne pas les PDF et ne recalibre pas les mesures.
