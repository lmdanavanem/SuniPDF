# Modifier la géométrie des mesures enregistrées

[English](MEASUREMENT_GEOMETRY.md) · [Mesures](MEASUREMENTS.fr.md)

Disponible dans [0.49.0](RELEASE_0.49.0.fr.md). [Preuves](qa/RELEASE_0.49.0.json).

Le parcours ci-dessous utilise la [disposition de lecture et des outils](READER_WORKSPACE.fr.md) publiée en [0.50.0](RELEASE_0.50.0.fr.md) ; les limites des géométries enregistrées restent inchangées.

## Utilisation

1. Ouvrez un PDF contenant une mesure enregistrée compatible et choisissez **Outils → Réviser et mesurer → Mesures**.
2. Dans **Données enregistrées** du panneau droit, ouvrez **Mesures enregistrées**, choisissez la mesure puis **Modifier la géométrie**.
3. Choisissez le numéro du point dans **Sommet**, puis déplacez sa poignée sur la page d’origine. La liste distingue aussi les sommets placés au même endroit. Les poignées gardent une taille constante à l’écran lorsque vous zoomez ; les points restent dans la page.
4. Pour une polyligne, un périmètre ou une surface polygonale, choisissez **Ajouter un sommet**, puis cliquez sur le segment qui doit recevoir le point. Une insertion réussie ajoute un point au brouillon et termine ce mode ; choisissez à nouveau **Ajouter un sommet** pour en placer un autre. **Arrêter l’ajout** désactive ce mode. Le segment de fermeture des périmètres et surfaces est inclus. Pour retirer un point, sélectionnez son numéro puis **Supprimer la sélection**.
5. Sélectionnez **Aperçu** pour examiner la mesure recalculée, puis **Appliquer à l’objet** pour la valider. **Ajuster les sommets** revient au brouillon si des corrections sont nécessaires.
6. Enregistrez le PDF normalement. **Annuler** dans l’historique revient sur la modification validée en une étape ; le bouton d’annulation du brouillon ou Échap abandonne les changements non validés.

Le type de mesure reste fixe. Une polyligne conserve au moins deux sommets ; un périmètre ou une surface polygonale, au moins trois. Ces formes acceptent au maximum 128 sommets. Les longueurs gardent deux points et les angles trois : leurs points peuvent être déplacés, mais l’insertion et la suppression sont indisponibles.

La modification géométrique conserve la calibration propre à la mesure, sa distance de référence, son unité et sa précision. Elle ne les remplace pas par les paramètres de page ou l’échelle courante de dessin. Utilisez l’éditeur distinct des paramètres de mesure pour modifier ces propriétés explicitement.

## Données et permissions

L'annotation conserve son identité, ses réponses, ses champs du registre et ses métadonnées de quantité. Les quantités mesurées et les totaux compatibles avec les prix unitaires sont recalculés à partir de la géométrie modifiée ; champs saisis, prix et base tarifaire restent inchangés. Les autres mesures et les paramètres de page restent indépendants.

Aperçu et Appliquer utilisent l'opération existante de modification contrôlée. Une source modifiée, un enregistrement non pris en charge, une annotation verrouillée ou l'absence de permission d'annotation empêchent la validation. Une géométrie invalide, notamment un polygone effondré ou auto-intersecté, doit être corrigée avant validation. Les changements de page, d'onglet ou de document invalident les brouillons et aperçus périmés.

## Périmètre

Utilisez **Sommet** pour désigner explicitement un point lorsque les poignées se superposent. Zoomez si des segments proches rendent l’insertion ambiguë. Le point est inséré sur un segment de la mesure sélectionnée ; il ne s’accroche pas aux traits PDF sous-jacents. Une géométrie invalide doit toujours être corrigée avant Appliquer.

Cette fonction modifie les mesures enregistrées compatibles une par une. Accrochage vectoriel, recalcul par lots et zones à échelles multiples restent des travaux distincts. Un plan raster n’acquiert pas d’accrochage vectoriel grâce à cette fonction. Les tests automatisés d’interaction ne constituent pas une qualification matérielle native ou d’accessibilité. AEC-002 et AEC-003 restent partiels.

La commande CLI existante `edit-measurement` reste disponible pour remplacer explicitement la géométrie dans une nouvelle copie PDF avec un enregistrement attendu ; voir le [guide des mesures](MEASUREMENTS.fr.md). Le parcours sur la page n'ajoute aucune commande CLI.
