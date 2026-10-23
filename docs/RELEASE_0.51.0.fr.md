# SuniPDF 0.51.0 — paquet de bureau Ubuntu

[English](RELEASE_0.51.0.md) · [Installation Ubuntu et limites](UBUNTU.fr.md)

[0.51.0 publiée en Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.51.0) le 2026-10-23, sous `lmdanavanem`.

- Interface Linux, entrée dans le menu des applications, association PDF et CLI installé.
- Icônes d'interface libres intégrées pour garder des commandes lisibles sans les polices Windows.
- Bibliothèque PDFium privée avec empreintes épinglées ; OCR local français/anglais via les paquets Ubuntu.
- Recherche de polices Linux, chemins récents sensibles à la casse, préférences XDG et récupération protégée entre instances.
- Notifications de mise à jour vers le téléchargement Ubuntu. L'installation utilise le gestionnaire de paquets du système.
- Indication explicite des fonctions nécessitant encore Windows ; maintien des fonctions Windows existantes.
- Ajout des paquets 0.50.0 publiés aux tests de migration Windows portables et MSI.

Le premier paquet Linux cible Ubuntu 22.04 et 24.04 en amd64. L'impression native, la signature et la confiance par certificat, l'import Office protégé et la bibliothèque persistante chiffrée de signatures visuelles restent réservés à Windows. Les signatures visuelles de session et les exports PDF vers Office restent disponibles. Linux ARM64 et les validations sur matériel physique sont hors du périmètre de cette version. Les paquets Windows restent non signés.


## Qualification

Source `f046c51df5d2caa31d4fa8710edd48471b241809`, PR [#31](https://github.com/lmdanavanem/SuniPDF/pull/31). Source exacte validée : [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879765536) · [linux](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879765566) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879777012) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39879764800). Windows : 781 tests réussis, 0 échec, 12 ignorés. Linux : 763 tests réussis, 0 échec, 7 ignorés. 1417 clés EN/FR. 43 scénarios portables ; 37 références MSI / 185 journaux. Huit fichiers téléchargés publiquement et vérifiés par SHA256. Aucune alerte Code Scanning ouverte.

La qualification Ubuntu utilise des conteneurs 22.04/24.04 propres, un utilisateur normal, Xvfb et Glow logiciel. Elle couvre le paquet installé, le PDF visible, l’enregistrement/réouverture, l’OCR français/anglais et la suppression/réinstallation du même candidat ; elle ne valide ni une migration depuis une version Linux antérieure ni le matériel physique. Linux ARM64 est hors périmètre. L’impression native, la signature et la confiance par certificat, l’import Office protégé et la bibliothèque persistante chiffrée de signatures visuelles restent réservés à Windows. Les signatures visuelles de session et les exports PDF vers Office restent disponibles. Les paquets Windows restent non signés ; Windows ARM64 est compilé sans exécution native. La mise à jour de Projects reste différée.

[Preuves](qa/RELEASE_0.51.0.json).
