# SuniPDF 0.35.0

## Changements

- Lissage facultatif des signatures dessinées, avec la même géométrie pour l’aperçu et le placement, et conservation des points bruts pour annuler ou comparer.
- Réduction facultative des ombres du papier pour les signatures importées ; chaque réglage est lié à l’aperçu et le placement utilise exactement les pixels préparés.
- Commande `prepare-signature-image` pour produire un PNG séparé en mode conservation, seuil global ou fond adaptatif, sans ouvrir de PDF.
- Limites explicites du dessin, gardes d’entrée/sortie et preuves indépendantes sur les images empaquetées, EXIF, rendu PDF et qpdf.
- Conservation des références de mise à jour précédentes et ajout des paquets publiés de la 0.34.0.

[Utilisation et limites](VISUAL_SIGNATURE_PREPARATION.fr.md).

## Validation

[0.35.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) sous `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) et [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) réussies : **606 tests, zéro échec**, 10 ignorés par défaut, **1114 clés EN/FR**, **20 scénarios d’image**, **14 scénarios d’inspection PDF**, **27 scénarios portables**, **21 références MSI/105 journaux**. CodeQL Rust/Python/Actions réussit avec zéro alerte ouverte. Les contrôles qpdf, lecteurs indépendants, polices, rendu DX12 logiciel, PDF, mutations et Office restent inclus.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; les suites image et inspection ont aussi été rejouées sur la CLI téléchargée. Le harnais conservé de la 0.32.0, avec module de mise à jour inchangé et vérifié, a téléchargé et contrôlé les **26,752,605 octets** du ZIP x64, sans installation sur le poste.

Signatures visuelles sans preuve d’identité ; le nettoyage peut supprimer l’encre pâle et doit être vérifié dans l’aperçu. Interaction native/tablette/Narrateur et photos clients encore à qualifier. Paquets non signés, ARM64 compilé uniquement et critères 1.0 ouverts. Projects reste reporté à la demande du propriétaire, sans synchronisation nouvelle annoncée.

[Preuves détaillées](qa/RELEASE_0.35.0.json).

## Limites

Une signature visuelle ne prouve ni une identité ni l’intégrité cryptographique. Le nettoyage cible l’encre sombre et fine sur papier clair et peut supprimer des marques pâles ; vérifier l’aperçu. L’interaction native souris/tablette/Narrateur et les photos de clients restent à qualifier.

Les paquets Windows restent non signés et sans clé indépendante de publication des mises à jour. ARM64 est compilé sans exécution sur matériel ARM64. L’impression physique, l’élargissement du corpus hostile et du fuzzing, les lecteurs tiers nommés et la conformité PDF/A/PDF/UA restent ouverts dans les [critères 1.0](RELEASE_1.0_CRITERIA.fr.md). Les modifications Projects restent reportées à la demande du propriétaire.
