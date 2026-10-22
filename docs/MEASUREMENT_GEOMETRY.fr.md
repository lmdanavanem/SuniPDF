# Modifier la géométrie des mesures enregistrées

[English](MEASUREMENT_GEOMETRY.md) · [Mesures](MEASUREMENTS.fr.md)

Cette version ajoute la modification des sommets sur la page pour les mesures SuniPDF enregistrées. Distribution et qualification figurent sur la [page de version 0.48.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.48.0).

## Utilisation

1. Ouvrez un PDF contenant une mesure enregistrée compatible et sélectionnez **Mesures**.
2. Dans **Mesures enregistrées**, choisissez la mesure puis **Modifier la géométrie**.
3. Déplacez un sommet existant sur sa page d'origine. Les poignées gardent une taille constante à l'écran lorsque vous zoomez. Les points restent dans les limites de la page.
4. Sélectionnez **Aperçu** pour examiner la mesure recalculée, puis **Appliquer** pour la valider. **Ajuster les sommets** revient au brouillon si des corrections sont nécessaires.
5. Enregistrez le PDF normalement. **Annuler** dans l'historique revient sur la modification validée en une étape ; le bouton d'annulation du brouillon ou Échap abandonne les changements non validés.

Longueurs, polylignes, périmètres, surfaces polygonales et angles conservent leur type et leur nombre de sommets. La modification géométrique conserve la calibration propre à la mesure, sa distance de référence, son unité et sa précision. Elle ne les remplace pas par les paramètres de page ou l'échelle courante de dessin. Utilisez l'éditeur distinct des paramètres de mesure pour modifier ces propriétés explicitement.

## Données et permissions

L'annotation conserve son identité, ses réponses, ses champs du registre et ses métadonnées de quantité. Les quantités mesurées et les totaux compatibles avec les prix unitaires sont recalculés à partir de la géométrie modifiée ; champs saisis, prix et base tarifaire restent inchangés. Les autres mesures et les paramètres de page restent indépendants.

Aperçu et Appliquer utilisent l'opération existante de modification contrôlée. Une source modifiée, un enregistrement non pris en charge, une annotation verrouillée ou l'absence de permission d'annotation empêchent la validation. Une géométrie invalide, notamment un polygone effondré ou auto-intersecté, doit être corrigée avant validation. Les changements de page, d'onglet ou de document invalident les brouillons et aperçus périmés.

## Périmètre

Cette fonction modifie les sommets existants des mesures enregistrées prises en charge. Ajout ou suppression de sommets, accrochage aux vecteurs, recalcul par lots et zones à échelles multiples restent des travaux distincts. Un plan raster n'acquiert pas d'accrochage vectoriel grâce à cette fonction. Les tests automatisés d'interaction ne constituent pas une qualification matérielle native ou d'accessibilité.

La commande CLI existante `edit-measurement` reste disponible pour remplacer explicitement la géométrie dans une nouvelle copie PDF avec un enregistrement attendu ; voir le [guide des mesures](MEASUREMENTS.fr.md). Le parcours sur la page n'ajoute aucune commande CLI.
