# SuniPDF 0.31.0

[English](RELEASE_0.31.0.md)

## Nouvelles fonctions

- **Comparer localement deux versions PDF enregistrées.** Alignez leurs pages physiques avec des décalages explicites, consultez les différences de texte extrait, dimensions et pixels, puis ouvrez les aperçus original/révision/différences rouges d’une paire. Une source modifiée est refusée avant aperçu.
- **Rechercher dans plusieurs PDF sélectionnés.** Recherche littérale avec casse optionnelle, résultats par fichier, extraits et boutons ouvrant les pages correspondantes. Les fichiers absents, invalides, protégés ou interdisant la copie restent signalés.
- **Automatiser ces opérations.** Les nouvelles commandes JSON `compare` et `search-files` partagent le même moteur borné. Une analyse partielle produit son rapport et le code de sortie 2. Les sources de mots de passe environnement/stdin restent disponibles.

## Améliorations

- Progression et annulation conservent les pages terminées. Les opérations en lecture seule préservent les sources, sans envoi, parcours de dossier ni index enregistré.
- Contrôles anglais/français, avec avertissement pour les paires sans texte extractible.
- Régressions de mise à jour portable/MSI étendues aux paquets publics 0.30.1.

## Limites prises en charge

Le [guide d’analyse locale](LOCAL_REVIEW.fr.md) définit l’alignement, les bornes, la résolution et les échecs.
Fichiers enregistrés uniquement ; sans OCR ni alignement automatique. Un aperçu de 768 pixels avec tolérance 16/255 ne prouve ni identité des PDF, équivalence sémantique, validité des signatures, suppression des données cachées ni conformité archivistique.
Le bureau signale les fichiers protégés ; la CLI accepte un même secret pour les entrées sélectionnées.
Les paquets Windows restent non signés ; les mises à jour reposent sur HTTPS GitHub et SHA256 du même canal. Une signature indépendante attend une clé fournie. ARM64 est compilé ; qualification matérielle et manuelle séparée.

## Validation et distribution

Publiée en [Latest stable](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.31.0) sous `lmdanavanem`, release `417463135`, source immuable `362c7f3979533000e454769e48a8cfbd4084dc0d`.

[CI principale](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280498112) et [workflow de release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39280650040) réussis : **582 tests**, zéro échec, 9 ignorés par défaut ; **1091 clés EN/FR**, 84 contrôles qpdf + 9 sorties indépendantes, 4 cas de polices, rendu logiciel DX12, 14 allers-retours Office, 4 PDF sous licence/744 pages, **23 scénarios portables**, **17 références MSI/85 journaux**. CodeQL Rust/Python/Actions réussit sans alerte ouverte après revue individuelle de quatre secrets de tests synthétiques.

Six fichiers publics et copies `dist/` vérifiés par SHA256 ; onze scénarios d’analyse locale et huit de sécurité réussissent avec la CLI empaquetée. Le harnais fraîchement compilé sur cette source a téléchargé et vérifié le ZIP x64 de **27,090,504 octets en 1.71 s**, sans installation. Projects : **43 cartes, 24 Done, 19 Todo, zéro In Progress** ; les deux Suggestions P2 sont livrées, les critères conservés et les trois filtres relus.

[Preuves détaillées](qa/RELEASE_0.31.0.json).
