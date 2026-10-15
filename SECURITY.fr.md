# Sécurité

[English](SECURITY.md)

SuniPDF traite des fichiers PDF, un format complexe. Si vous découvrez une vulnérabilité (fichier
provoquant un plantage exploitable, contournement d'une protection par mot de passe, caviardage incomplet,
fuite de données hors de la machine), merci de **ne pas ouvrir d'issue publique** : écrivez à
l'adresse indiquée sur le profil GitHub du mainteneur ([@lmdanavanem](https://github.com/lmdanavanem))
ou utilisez la fonction « Report a vulnerability » de GitHub si elle est activée sur le dépôt.

Vous recevrez un accusé de réception sous 7 jours. Les correctifs sont publiés dans une nouvelle version
et décrits dans `CHANGELOG.md`.

Périmètre : versions publiées sur la page Releases. SuniPDF traite les PDF localement ; la recherche de mises à jour contacte GitHub sans envoyer de document ; le moteur de rendu est PDFium (chargé dynamiquement, sans JavaScript ni XFA).
