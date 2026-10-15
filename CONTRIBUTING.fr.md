# Contribuer à SuniPDF

[English](CONTRIBUTING.md)

Merci de votre intérêt. SuniPDF est un logiciel libre (MIT ou Apache-2.0, au choix) créé et maintenu par
Emanuel DE ALMEIDA ([@lmdanavanem](https://github.com/lmdanavanem)).

## Avant de commencer
- Lisez `docs/ENGINEERING_NOTES.md` (commandes, garde-fous, patrons de code, pièges des bibliothèques) et
  `docs/ARCHITECTURE.md`.
- Ouvrez une *issue* avant une modification importante pour en discuter ; les petites corrections peuvent
  arriver directement en *pull request*.

## Règles du projet
- Traitement PDF local, sans télémétrie : aucun PDF ne quitte la machine. La recherche de versions contacte GitHub et peut être désactivée.
- Interface entièrement traduisible : aucune chaîne visible en dur dans `apps/`, toute clé va dans
  `assets/i18n/fr-FR.json` **et** `assets/i18n/en-US.json`.
- Moteur (`crates/sunipdf-core`) découplé de l'interface et de la CLI ; un seul thread touche PDFium.
- Sauvegardes atomiques, jamais d'écrasement silencieux ; pas de bouton sans fonction réelle ; une
  fonction non terminée reste dans la feuille de route.
- Chaque fonctionnalité arrive avec ses tests (relecture du fichier produit, pixels du rendu) et sa
  documentation (`CHANGELOG.md`, guides utilisateur, `docs/KNOWN_ISSUES.md` pour les limites).

## Garde-fous (tous verts avant une pull request)
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-i18n.ps1
```

## Pull requests
- Une PR = un sujet. Décrivez le comportement avant/après, les tests ajoutés et ce qui n'est pas couvert.
- Les messages de commit sont en français ou en anglais, à l'impératif, courts.
- En contribuant, vous acceptez que votre contribution soit publiée sous les deux licences du projet.
