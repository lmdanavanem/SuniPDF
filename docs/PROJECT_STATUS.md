# État du projet — mis à jour le 2026-10-13

Règle : une case cochée = code présent **et** commande exécutée avec résultat observé sur ce poste
(Windows 11 Pro 26300, Rust 1.99.0, cible `x86_64-pc-windows-gnu`, PDFium 7881).

## Phase 0 — Audit et planification : terminée
- [x] `docs/PDFCRAFT_AUDIT.md` (faits vérifiés / hypothèses / futur), `FEATURE_MATRIX.md`, `ARCHITECTURE.md`,
      `TECH_STACK.md`, `ROADMAP.md`, `AGENT_STRATEGY.md`, `I18N.md`, `DECISIONS.md`, `KNOWN_ISSUES.md`, `THIRD_PARTY_NOTICES.md`.
- [x] 12 agents enrichis dans `.claude/agents/` (agent `architect` invoqué pour cette tâche).

## Phase 1 — Fondations : en cours
- [x] Workspace Rust (`sunipdf-core`, `sunipdf-i18n`, `sunipdf-desktop`, `sunipdf-cli`), `cargo build` OK.
- [x] Moteur PDFium intégré (`pdfium-render` 0.9.4, DLL chargée dynamiquement, 4 emplacements de recherche).
- [x] Ouvrir / afficher / naviguer (clavier, barre d'outils, champ page) / zoom (ajuster largeur, page, paliers) / fermer.
- [x] Documents protégés : boîte de mot de passe, message « mot de passe incorrect ».
- [x] Glisser-déposer d'un `.pdf`, ouverture par argument de ligne de commande, titre de fenêtre.
- [x] États UI : vide, moteur en démarrage, ouverture, rendu en cours (spinner), erreur (bandeau), moteur absent.
- [x] i18n fr-FR / en-US : 76 clés, changement à chaud, préférence persistée (écriture atomique), thème clair/sombre.
- [x] Rendu hors thread UI (thread moteur unique, générations, un rendu en vol).
- [x] CLI : `info` (JSON), `render` (PNG), `commands` (registre), codes d'erreur stables, codes de sortie 3/4/5.
- [x] SUNI-014 (moteur + CLI) Fusion, extraction, division, rotation, suppression de page, enregistrement atomique (`sunipdf-core::pages`, `write_atomic` partagé avec les préférences) ; CLI `merge`, `extract`, `split`, `rotate` ; 4 tests. **UI non branchée** (menu Pages à venir).
- [x] SUNI-013 Recherche plein texte (Ctrl+F, barre de recherche, casse, occurrence suivante/précédente, surbrillance sur la page) et extraction de texte ; CLI `text` et `search` ; 1 test moteur.
- [x] SUNI-011 Cache LRU de rendu plafonné (256 Mo, `sunipdf-core::cache`), pré-rendu des pages voisines, panneau de miniatures virtualisé (F4), 2 tests cache.
- [x] SUNI-010 Registre de commandes (`sunipdf-core::command`) : 15 commandes, menus et raccourcis routés par `SuniApp::execute`, 3 tests.
- [x] Tests exécutés : `cargo test --workspace` → voir « Dernière exécution ».
- [x] Test de fumée : `sunipdf.exe two-pages.pdf` démarre et reste actif (fenêtre non inspectée visuellement par l'agent).
- [ ] Validation visuelle par l'utilisateur sur son écran (rendu, DPI, menus).
- [ ] Compilation MSVC (cible de référence) : impossible sur ce poste sans Build Tools.
- [ ] Icône / ressources de version Windows.
- [ ] CI GitHub Actions : fichier écrit, **jamais exécuté**.

## Dernière exécution (2026-10-13, cible GNU, `-j 2`)
| Commande | Résultat |
|---|---|
| `cargo test -p sunipdf-core` | 16/16 OK (2 pages + 2 cache + 2 registre + fusion/extraction/division + rotation/suppression/enregistrement + texte/recherche + 7 moteur : fixture 2 pages, chiffré RC4 avec/sans mot de passe, fichier invalide, fichier absent, page hors limites, rendu trop grand, DLL absente) |
| `cargo test -p sunipdf-i18n` | 6/6 OK (parité des clés, valeurs non vides, paramètres identiques, repli, codes, substitution) |
| `cargo test -p sunipdf-desktop` | 5/5 OK (clés i18n utilisées, clés du registre, clés d'erreur, préférences atomiques, utilitaires) |
| `cargo test -p sunipdf-cli` | 1/1 OK (analyse des plages de pages) |
| `sunipdf-cli info/render/commands/text/search` | JSON correct ; PNG 1224×1584 vérifié visuellement ; texte extrait et 2 occurrences avec rectangles |
| `cargo clippy -p sunipdf-core -p sunipdf-i18n -p sunipdf-cli --all-targets -D warnings` | OK, 0 avertissement. **Pas exécutable pour `sunipdf-desktop` sur ce poste** : rustc GNU abandonne (0xC0000409) sur le crate `windows` 0.62 faute de mémoire, même en `-j 1` (à vérifier en CI MSVC) |
| `cargo fmt --all -- --check` | OK |
| `scripts\check-i18n.ps1` | OK : 76 clés bilingues |


## Non réalisé (ne pas confondre avec « fait »)
Onglets, affichage continu, sélection/copie de texte, signets, toute édition, interface des opérations de pages (moteur et CLI faits), OCR, conversions, chiffrement à l'écriture, signatures, MSI, ARM64, benchmarks.

## Journal
- 2026-10-13 : audit PDFCraft, choix PDFium (D005), squelette → tranche verticale 1, environnement GNU faute de MSVC (D008),
  wgpu désactivé (D010), fixtures générées (`scripts/make_fixtures.py`), dépôt git initialisé.
