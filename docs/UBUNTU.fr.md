# SuniPDF sous Ubuntu

[English](UBUNTU.md) · [Guide utilisateur](USER_GUIDE.fr.md)

Le paquet Ubuntu cible **Ubuntu 22.04 et 24.04, amd64 (x86_64)**. Il installe l'application de bureau et `sunipdf-cli`. Linux ARM64 n'est pas inclus dans ce premier paquet. La [page de la version](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.51.0) présente les téléchargements disponibles et les résultats de validation.

## Installation

Téléchargez `SuniPDF-0.51.0-linux-amd64.deb` et son fichier `.sha256` depuis la page de la version. Dans le dossier de téléchargement :

```sh
sha256sum --check SuniPDF-0.51.0-linux-amd64.deb.sha256
sudo apt install ./SuniPDF-0.51.0-linux-amd64.deb
```

APT installe les dépendances système déclarées. Une connexion Internet peut être nécessaire pendant cette installation. L'édition des PDF et l'OCR fonctionnent ensuite localement.

Lancez **SuniPDF** depuis le menu des applications, ou exécutez :

```sh
sunipdf /chemin/vers/document.pdf
sunipdf-cli --help
```

Le paquet déclare une application capable d'ouvrir les PDF. Il ne remplace pas automatiquement votre lecteur PDF par défaut.

## Fonctions disponibles

L'application Linux partage le moteur PDF, le lecteur, les annotations, les mesures, les tableaux de quantités, les opérations sur les pages, les formulaires et les exports PDF avec l'application Windows. Les permissions des documents, les aperçus et les protections de sauvegarde restent applicables.

L'OCR utilise Tesseract installé par Ubuntu et ses langues anglaise et française. Les polices d'édition PDF sont recherchées dans les dossiers de polices Linux. Les dialogues utilisent le portail de fichiers du bureau et son mécanisme de repli disponible.

## Limites de la plateforme

Les fonctions suivantes nécessitent actuellement Windows et sont indisponibles dans l'interface Ubuntu :

- L'impression native depuis SuniPDF. Ouvrez le PDF enregistré dans une autre application installée pour l'imprimer.
- La signature par certificat et la vérification de confiance par le système. L'inspection des structures de signature n'établit ni leur validité cryptographique ni leur confiance.
- L'import de documents Office par le processus de conversion Windows protégé. Les exports PDF vers Office restent disponibles dans leurs limites documentées.
- Le stockage persistant chiffré des signatures visuelles. Dessiner, importer et placer une signature visuelle pendant la session reste possible.

Les actions interdites par les protections existantes des documents signés restent interdites. Une signature visuelle est une annotation et ne constitue pas une signature numérique par certificat.

## Préférences, récupération et mises à jour

Les préférences et la bibliothèque d'annotations utilisent `$XDG_CONFIG_HOME/sunipdf`, ou `$HOME/.config/sunipdf` si cette variable est absente. La récupération utilise `$XDG_STATE_HOME/sunipdf/recovery`, ou `$HOME/.local/state/sunipdf/recovery`. Seuls les chemins XDG absolus sont acceptés. Les sessions de récupération actives utilisent un verrou de fichier du système.

La notification de mise à jour ouvre la page officielle avec **Télécharger la mise à jour**. Téléchargez le nouveau `.deb`, vérifiez son empreinte, puis installez-le avec `sudo apt install ./<paquet-téléchargé>.deb`. L'application Linux ne lance pas l'installateur Windows et ne demande pas silencieusement des privilèges supplémentaires. Cette distribution ne configure pas de dépôt de mises à jour APT.

Pour supprimer l'application :

```sh
sudo apt remove sunipdf
```

La suppression du paquet conserve vos PDF, préférences personnelles et fichiers de récupération.

## Portée des validations

La validation du paquet installe le même `.deb` compilé sous Ubuntu 22.04 et 24.04 et contrôle les dépendances déclarées, les commandes installées, les opérations PDF, l'OCR et le lancement de l'interface sur un affichage virtuel. Les preuves de publication identifient la source réelle et les contrôles terminés. Ces essais ne qualifient pas tous les processeurs graphiques physiques, environnements de bureau, outils d'accessibilité ou imprimantes.
