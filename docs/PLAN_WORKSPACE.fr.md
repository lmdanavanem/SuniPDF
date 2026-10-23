# Espace des plans : échelles précises, lots et tables réactives

[English](PLAN_WORKSPACE.md) · [Périmètre 0.60](RELEASE_0.60.0.fr.md) · [Mesures](MEASUREMENTS.fr.md)

## Échelles et accrochage vectoriel

Ouvrez **Outils → Réviser et mesurer → Mesures**. Calibrez deux points de distance connue et confirmez la distance réelle positive. Une valeur décimale utilise l'unité choisie. Les notations `5' 6 1/2"`, `5 ft 6 in` ou `6 1/2 in` sont converties dans cette unité ; signes mélangés, dénominateurs nuls et texte ambigu sont refusés. Contrôlez une seconde dimension connue avant d'utiliser les quantités.

Activez l'accrochage vectoriel pour afficher l'extrémité ou l'intersection de segments droits éligible la plus proche. La tolérance suit le zoom. Les poignées de courbes, rectangles de texte/image et scans ne deviennent pas des points de mesure. Rognage, rotation, matrices des objets/formulaires imbriqués et découpes convexes compatibles sont appliqués. La rencontre d'un trait avec une découpe n'invente pas une nouvelle extrémité. Les découpes courbes/non convexes incompatibles sont omises et signalées ; une géométrie excessive ou un voisinage dense peut refuser l'accrochage. Utilisez alors des points manuels explicites.

Pour créer une zone nommée, calibrez sa référence à l'intérieur de la zone, puis placez ses deux coins opposés. Enregistrez la zone, puis le PDF. Chaque zone conserve une identité stable et une calibration indépendante de l'échelle de page et des annotations enregistrées. L'échelle active reste visible. La sélection automatique n'accepte qu'une zone contenant la géométrie sans ambiguïté ; chevauchements et traversées imposent un choix explicite. Choisissez une zone pour contraindre le tracé à ses limites, ou l'échelle de page explicitement lorsque c'est adapté. Modifier/supprimer une zone concerne les futurs tracés seulement.

Le champ de page borné `SuniPDFMeasurementViewports` conserve les coordonnées utilisateur PDF, limites de page et empreinte du contenu décodé. Il suit la rotation et les opérations compatibles. Contenu/limites modifiés, données inconnues ou malformées sont refusés plutôt qu'écrasés. Suppression des métadonnées, assainissement et caviardage retirent ce champ privé. Ces contrôles n'authentifient pas toutes les révisions : vérifiez le plan vous-même.

## Recalibrer un lot choisi

1. Confirmez l'échelle de référence voulue. Ouvrez le recalcul par lots et choisissez la plage de pages physiques.
2. Lisez les pages sélectionnées. Choisissez les mesures attendues et le remplacement éventuel des échelles de page. Les angles sont exclus car ils n'ont pas d'échelle de longueur.
3. Comparez les valeurs avant/après. Chaque mesure conserve unité, précision, géométrie, identité, réponses et champs de quantités ; les surfaces suivent le carré du rapport linéaire.
4. Appliquez une seule fois. Les enregistrements sont contrôlés avant un remplacement unique. Document modifié, page ultérieure invalide, annotation protégée/incompatible ou PDF signé entraînent un refus sans recalcul partiel des pages précédentes.
5. Un seul **Annuler** restaure le lot complet ; enregistrez le PDF pour conserver le résultat. Les échelles des zones nommées restent inchangées.

Les échelles de page exigent le droit de modification, les mesures enregistrées le droit d'annotation. Le chiffrement AES autorisé est conservé. Limites : 512 pages, 4096 mesures sélectionnées, 128 Mio par document, 128 sommets par mesure et 64 zones par page. Les références doivent tenir sur chaque page choisie ; ne réutilisez pas la même référence sur des feuilles d'échelles différentes sans les contrôler.

## Tables et lecture en arrière-plan

Le registre et les quantités peuvent être ancrés sous le PDF ou flotter dans une fenêtre. Leur position et la hauteur du panneau inférieur sont mémorisées. La sélection d'une ligne suit l'annotation réelle et ouvre ses propriétés. Les tables longues n'affichent que les lignes visibles ; filtres, tris et totaux inchangés réutilisent leurs résultats.

Recherche et inventaire progressent page par page avec progression/annulation ; les demandes de pages visibles passent entre ces tâches. L'annulation est coopérative : un appel au moteur PDF se termine avant la page suivante. La disposition est mise en cache selon la révision du document et les paramètres de lecture, pour éviter sa reconstruction lors d'un simple défilement. Les mesures enregistrées établissent la latence observée, pas une garantie universelle de vitesse.

## CLI

```text
sunipdf-cli measurement-viewports input.pdf --page 1
sunipdf-cli set-measurement-viewports input.pdf --page 1 --expected current-regions.json --regions next-regions.json --out regions.pdf
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json
sunipdf-cli recalibrate-measurements input.pdf --requests recalibration.json --out recalibrated.pdf
sunipdf-cli measurement-snap input.pdf --page 1 --point "210,320" --tolerance 5
```

`expected` et `regions` contiennent des tableaux JSON. Une zone comporte exactement `id`, `name`, `bounds` (`[gauche,bas,droite,haut]`) et `calibration` (`points`, `known_distance`, `unit`). Les points sont dans la page affichée, origine bas-gauche. Reprenez le tableau `regions` de la lecture comme état attendu exact. Chaque fichier est limité à 128 Kio.

`recalibration.json` est un tableau d'objets avec exactement `page` (à partir de 1), `calibration`, `update_page_default` (booléen) et `measurements` (enregistrements complets attendus de l'export `measurements`, identités stables comprises). Limite : 4 Mio. Un tableau de mesures vide signifie aucune mesure, jamais toutes. Sans `--out`, seule la prévisualisation est calculée ; avec `--out`, le plan validé produit une nouvelle copie. Les destinations existantes et le fichier original sont refusés même avec `--overwrite`. Lecture des zones, export des accrochages et aperçu des valeurs exigent le droit de copie. Utilisez `--password-env` ou `--password-stdin` pour les identifiants.

La qualification automatisée utilise une géométrie synthétique connue et les PDF publics/sous licence conservés. Les plans privés de projets sont reportés à la demande du propriétaire. Matériel physique, accessibilité, interopérabilité complète et autres critères 1.0 restent ouverts.
