# SuniPDF 0.44.0 — modification des mesures enregistrées

[English](RELEASE_0.44.0.md) · [Guide des mesures](MEASUREMENTS.fr.md)

État : publiée en Latest stable [0.44.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.44.0), source `7e95078127beb81b90a9eac32e6fde0fd8e9bf50`. La version précédente et ses preuves restent [0.43.0](RELEASE_0.43.0.fr.md) · [0.43.0 evidence](qa/RELEASE_0.43.0.json).

## Audit avant cette version

| Constat | Correction livrée |
|---|---|
| Le dialogue manuel fournissait une référence arbitraire de 10 m sur 100 points PDF. | Aucune distance positive implicite ; une calibration explicite valide est nécessaire avant aperçu/application. |
| Les mesures enregistrées pouvaient être consultées ou réutilisées, sans être modifiées. | Un éditeur explicite modifie distance de référence, unité et précision de la mesure choisie, avec aperçu puis application. |
| L'inspecteur copiait toute la liste et générait le JSON des résultats à chaque image. | Données et libellés calculés sont partagés en mémoire ; la pagination reste limitée à 50 lignes. |
| Une échelle temporaire pouvait masquer le défaut enregistré sans retour direct. | Utiliser l'échelle enregistrée ou Effacer l'échelle temporaire agit uniquement sur la session de la page courante. |
| Les exports de mesures ne contrôlaient pas systématiquement le droit de copie, y compris les réponses détaillées des commandes de modification. | Les commandes de lecture/export exigent le droit de copie. La création/modification autorisée sans ce droit renvoie `measurement: null` et `export_unavailable: true`. |
| Le redimensionnement du contenu et des annotations changeait les rectangles des mesures sans actualiser leur géométrie privée. | Ce mode refuse les pages contenant des mesures compatibles ou importées reconnues avant toute application. |
| Remplacer une mesure exige davantage de contrôles que l'afficher. | L'édition exige un enregistrement attendu inchangé, une identité stable unique, des métadonnées/apparences compatibles et le droit d'annotation. Les cibles verrouillées, partagées, périmées ou modifiées ailleurs sont refusées. |

## Application

Ouvrez **Mesures → Mesures enregistrées → Modifier la mesure**. Modifiez la distance de référence connue, l'unité ou la précision de 0 à 4 décimales. Changer l'unité convertit physiquement la référence ; changer la distance recalcule uniquement cette mesure. Pour un angle, seule la précision est proposée. Sommets et points de référence restent fixes dans l'éditeur de l'application.

Vérifiez l'**Aperçu** rendu, puis cliquez sur **Appliquer**. Le remplacement conserve identité, auteur, réponses et champs du tableau des annotations. **Annuler** conserve le PDF intact ; une application réussie correspond à une étape Annuler/Rétablir. Enregistrez le PDF pour conserver le changement. Les échelles de page et les autres mesures ne sont pas recalculées.

**Utiliser l'échelle enregistrée** abandonne la calibration temporaire lorsqu'un défaut valide existe. Sinon, **Effacer l'échelle temporaire** retire uniquement cette calibration. Ces actions ne modifient ni le PDF ni l'échelle de session d'une autre page.

## CLI

`edit-measurement` exige `--page`, `--index` et `--expected-record`, contenant un enregistrement complet obtenu par `measurements`. Elle peut modifier type, sommets, calibration, unité ou précision. Une calibration explicite exige `--calibration` et `--known-distance` ; `--use-page-calibration` sélectionne le défaut de la page. Transformer un angle en mesure étalonnée exige une calibration physique explicite. Un angle peut être créé sans échelle de page.

Les écritures exigent une nouvelle copie `--out`, même avec `--overwrite`. Destinations existantes et originaux restent protégés. Chiffrement compatible et permissions sont conservés. L'export JSON détaillé exige le droit de copie ; le droit d'annotation reste nécessaire indépendamment pour modifier. [Exemples et limites](MEASUREMENTS.fr.md#cli-et-données).

## Validation et limites restantes

Le périmètre de validation comprend interactions réelles avec les contrôles egui, conversion d'unités, aperçu/annulation, cibles périmées, remplacement sans doublon, Annuler/Rétablir, enregistrement/réouverture, copies chiffrées, exports restreints et contrôles qpdf indépendants. Source exacte validée: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718850233) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718862137) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718850516). 702 tests réussis, 10 ignorés par défaut. 1250 clés EN/FR. 36 scénarios portables. 30 références MSI. 150 journaux MSI. Aucune alerte Code Scanning ouverte. Six téléchargements publics et copies dist vérifiés par SHA256. 21 appels CLI vérifient conversion, précision, recalibration, géométrie, refus périmé, identité, échelle de page, réouverture et angle sans calibration. Valeur finale : 40 m ; autres mesures et originaux conservés. [Preuves](qa/RELEASE_0.44.0.json).

Déplacement des sommets à la souris, recalcul par lot, accrochage vectoriel, vues multi-échelles, volumes et qualification dimensionnelle sur plans réels restent ouverts. L'édition concerne les mesures compatibles de documents jusqu'à 128 Mio ; les structures inconnues ou modifiées ailleurs sont préservées par refus. Le redimensionnement des pages mesurées n'est pas réparé silencieusement. Le local reste gratuit et open source. Signature des paquets, ARM64 natif, accessibilité native, impression physique et critères 1.0 restent ouverts.

Séquence locale prévue : 0.44 édition et corrections, 0.45 bibliothèque d'annotations réutilisables, 0.46 colonnes/totaux/export de métrés, 0.47 superpositions et révisions. Chaque étape exige son propre périmètre et ses preuves ; cette version ne termine pas les cartes AEC générales. [Roadmap](ROADMAP.fr.md).
