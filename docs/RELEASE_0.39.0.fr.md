# SuniPDF 0.39.0

## Changements

- Modèles locaux nommés pour filigranes texte/image, numéros et six zones d’en-tête/pied.
- Charger un brouillon, enregistrer sous un nom unique ou supprimer explicitement ; 16 modèles au maximum.
- Préférences atomiques, chargement borné et isolation des modèles mal formés.
- Portée des pages à vérifier à chaque usage ; régressions de paquets, sécurité et migrations conservées.
- Vérification de version Office sans interface avec un budget annulable de 30 secondes au démarrage à froid ; limites de conversion et gardes de sécurité conservées.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

La qualification Windows, CodeQL, les paquets et la publication sur la source finale restent à exécuter. Les régressions sans fenêtre couvrent quatre modes, compatibilité, valeurs invalides, échec d’écriture et annulation. L’interaction Windows native reste à qualifier.

## Limites

Les images référencent leur chemin d’origine : garder le fichier disponible. Vérifier la portée des pages. Enregistrer/supprimer un modèle persiste indépendamment de l’annulation du document. Pas de macro, cloud, import/export CLI ou stockage d’identité de signature. Paquets non signés et ARM64 uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects reste différé à la demande du propriétaire.
