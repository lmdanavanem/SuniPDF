# SuniPDF 0.37.0

## Changements

- Filigranes image dans la fenêtre Filigrane et la commande CLI `image-watermark`.
- Transparence, largeur proportionnelle, angle, sept positions et ajustement à la page affichée.
- Modification atomique des pages choisies, une annulation dans l’application, nouvelle copie en CLI et conservation du chiffrement autorisé.
- Régression indépendante sur géométrie, pixels, refus et protection avec qpdf ; paquets 0.36.0 ajoutés aux migrations.

[Utilisation et limites](PAGE_DECORATIONS.fr.md).

## Validation

La qualification Windows, CodeQL, les paquets et la publication sur la source finale restent à exécuter. Les contrôles locaux du moteur et de la CLI sur fichiers générés passent ; l’interaction native reste à qualifier.

## Limites

Le filigrane image est un contenu de premier plan modifiable. Aucun arrière-plan, mosaïque, DRM ou contrôle de signature n’est promis. Les paquets restent non signés ; ARM64 est uniquement compilé en croisé. Les autres [critères 1.0](RELEASE_1.0_CRITERIA.fr.md) restent ouverts. Projects demeure différé à la demande du propriétaire.
