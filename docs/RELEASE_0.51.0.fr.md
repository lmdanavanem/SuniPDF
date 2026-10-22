# SuniPDF 0.51.0 — paquet de bureau Ubuntu

[English](RELEASE_0.51.0.md) · [Installation Ubuntu et limites](UBUNTU.fr.md)

Cette version ajoute un `.deb` Ubuntu amd64 aux paquets Windows. Les fichiers publiés et la validation de leur source exacte sont suivis sur la [page 0.51.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.51.0).

- Interface Linux, entrée dans le menu des applications, association PDF et CLI installé.
- Bibliothèque PDFium privée avec empreintes épinglées ; OCR local français/anglais via les paquets Ubuntu.
- Recherche de polices Linux, chemins récents sensibles à la casse, préférences XDG et récupération protégée entre instances.
- Notifications de mise à jour vers le téléchargement Ubuntu. L'installation utilise le gestionnaire de paquets du système.
- Indication explicite des fonctions nécessitant encore Windows ; maintien des fonctions Windows existantes.
- Ajout des paquets 0.50.0 publiés aux tests de migration Windows portables et MSI.

Le premier paquet Linux cible Ubuntu 22.04 et 24.04 en amd64. L'impression native, la signature et la confiance par certificat, l'import Office protégé et la bibliothèque persistante chiffrée de signatures visuelles restent réservés à Windows. Les signatures visuelles de session et les exports PDF vers Office restent disponibles. Linux ARM64 et les validations sur matériel physique sont hors du périmètre de cette version. Les paquets Windows restent non signés.
