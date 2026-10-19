# SuniPDF 0.35.0

## Changements

- Lissage facultatif des signatures dessinées, avec la même géométrie pour l’aperçu et le placement, et conservation des points bruts pour annuler ou comparer.
- Réduction facultative des ombres du papier pour les signatures importées ; chaque réglage est lié à l’aperçu et le placement utilise exactement les pixels préparés.
- Commande `prepare-signature-image` pour produire un PNG séparé en mode conservation, seuil global ou fond adaptatif, sans ouvrir de PDF.
- Limites explicites du dessin, gardes d’entrée/sortie et preuves indépendantes sur les images empaquetées, EXIF, rendu PDF et qpdf.
- Conservation des références de mise à jour précédentes et ajout des paquets publiés de la 0.34.0.

[Utilisation et limites](VISUAL_SIGNATURE_PREPARATION.fr.md).

## Validation

La CI Windows sur la révision finale, CodeQL et la qualification des paquets restent à exécuter. Cette note préparatoire ne revendique ni publication, ni interaction native avec l’interface, ni installation sur le poste.

## Limites

Une signature visuelle ne prouve ni une identité ni l’intégrité cryptographique. Le nettoyage cible l’encre sombre et fine sur papier clair et peut supprimer des marques pâles ; vérifier l’aperçu. L’interaction native souris/tablette/Narrateur et les photos de clients restent à qualifier.

Les paquets Windows restent non signés et sans clé indépendante de publication des mises à jour. ARM64 est compilé sans exécution sur matériel ARM64. L’impression physique, l’élargissement du corpus hostile et du fuzzing, les lecteurs tiers nommés et la conformité PDF/A/PDF/UA restent ouverts dans les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md). Les modifications Projects restent reportées à la demande du propriétaire.
