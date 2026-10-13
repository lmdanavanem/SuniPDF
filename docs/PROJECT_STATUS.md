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
- [x] SUNI-022/023 Signets (panneau F6, navigation), propriétés du document (Ctrl+D : métadonnées éditables, version, taille, chiffrement, permissions), protection par mot de passe AES-128/AES-256 avec permissions (dialogue + CLI `protect`), retrait de la protection (`unprotect`), suppression des métadonnées (`strip-metadata`), `set-metadata`, `properties`, `bookmarks` en CLI ; moteur `sunipdf-core::security` (lopdf pour l'écriture, PDFium pour la relecture) ; 2 tests moteur dont un aller-retour chiffrement/déchiffrement vérifié par PDFium.
- [x] SUNI-021 Annotations : sélection de texte par glissement et copie (Ctrl+C), outils surligner / souligner / barrer / note (palette flottante, raccourcis V H U X N), panneau Commentaires (F7, liste par page, suppression), catégorie « Annoter », toutes annulables ; moteur `sunipdf-core::annot` (PDFium génère les apparences) ; 1 test moteur avec vérification du rendu.
- [x] SUNI-014c Annuler/Rétablir par instantanés (Ctrl+Z / Ctrl+Y, 50 niveaux, 512 Mo max), insertion de pages depuis un autre PDF, déplacement de pages (glisser-déposer dans les miniatures et dans la vue Organiser, boutons avant/arrière), vue **Organiser les pages** (F8 : grille, sélection multiple clic/Ctrl/Maj, actions groupées), variable `SUNIPDF_STARTUP_COMMANDS` pour l'automatisation des captures ; CLI `insert`, `move` ; 1 test moteur (insertion, déplacement, instantané, chiffré).
- [x] SUNI-020 (UI) Coquille façon PDFCraft : barre d'onglets (accueil, document, + Ouvrir, langue, aide, thème), barre d'outils (Menu, catégories Tous les outils / Lire / Pages, recherche de commandes), panneau « Tous les outils », page d'accueil (cartes, fichiers récents persistés, confidentialité), rail droit (miniatures, recherche, page, zoom, ajustements, rotation), palette de commandes Ctrl+K, polices système Segoe UI + Segoe Fluent Icons chargées à l'exécution, thèmes clair/sombre retravaillés. Captures vérifiées par `scripts/dev-screenshot.ps1`.
- [x] SUNI-014b (UI) Menu Pages (pivoter à droite/gauche, supprimer la page), Enregistrer / Enregistrer sous (Ctrl+S, Ctrl+Maj+S), état « modifié » dans le titre, invite Enregistrer / Ne pas enregistrer / Annuler avant fermeture, ouverture ou quitter (y compris la croix de la fenêtre). 20 commandes au registre.
- [x] SUNI-014 (moteur + CLI) Fusion, extraction, division, rotation, suppression de page, enregistrement atomique (`sunipdf-core::pages`, `write_atomic` partagé avec les préférences) ; CLI `merge`, `extract`, `split`, `rotate` ; 4 tests.
- [x] SUNI-013 Recherche plein texte (Ctrl+F, barre de recherche, casse, occurrence suivante/précédente, surbrillance sur la page) et extraction de texte ; CLI `text` et `search` ; 1 test moteur.
- [x] SUNI-011 Cache LRU de rendu plafonné (256 Mo, `sunipdf-core::cache`), pré-rendu des pages voisines, panneau de miniatures virtualisé (F4), 2 tests cache.
- [x] SUNI-010 Registre de commandes (`sunipdf-core::command`) : 40 commandes, menus et raccourcis routés par `SuniApp::execute`, 3 tests.
- [x] Tests exécutés : `cargo test --workspace` → voir « Dernière exécution ».
- [x] Test de fumée : `sunipdf.exe two-pages.pdf` démarre et reste actif (fenêtre non inspectée visuellement par l'agent).
- [x] Captures d'écran réelles (accueil, document, palette) vérifiées par l'agent via `scripts/dev-screenshot.ps1` ; validation finale par l'utilisateur à faire.
- [x] Compilation MSVC (cible de référence) : Build Tools installés le 13/10 au soir, `cargo test --workspace` et `cargo clippy --workspace --all-targets -D warnings` verts.
- [ ] Icône / ressources de version Windows.
- [ ] CI GitHub Actions : fichier écrit, **jamais exécuté**.

## Dernière exécution (2026-10-13 soir, cible **MSVC**, `-j 2`)
| Commande | Résultat |
|---|---|
| `cargo test -p sunipdf-core` | 22/22 OK (1 sécurité + 1 annot + 2 pages + 2 cache + 2 registre + signets/propriétés + protection/métadonnées + sélection/annotations + fusion/extraction/division + rotation/suppression/enregistrement + insertion/déplacement/instantanés + texte/recherche + 7 moteur : fixture 2 pages, chiffré RC4 avec/sans mot de passe, fichier invalide, fichier absent, page hors limites, rendu trop grand, DLL absente) |
| `cargo test -p sunipdf-i18n` | 6/6 OK (parité des clés, valeurs non vides, paramètres identiques, repli, codes, substitution) |
| `cargo test -p sunipdf-desktop` | 5/5 OK (clés i18n utilisées, clés du registre, récents dédupliqués, clés d'erreur, préférences atomiques, utilitaires) |
| `cargo test -p sunipdf-cli` | 1/1 OK (analyse des plages de pages) |
| `sunipdf-cli info/render/commands/text/search/merge/rotate/extract/split` | JSON correct ; PNG vérifié visuellement ; texte et occurrences ; fusion 4 pages, rotation pages 1 et 4 (tailles inversées), extraction 2-3, division en 2 fichiers |
| `cargo clippy --workspace --all-targets -- -D warnings` | OK, 0 avertissement (MSVC) |
| `cargo fmt --all -- --check` | OK |
| `scripts\check-i18n.ps1` | OK : 76 clés bilingues |


## Non réalisé (ne pas confondre avec « fait »)
Onglets multiples, affichage continu, formes et dessin libre, édition du contenu existant, OCR, conversions, chiffrement à l'écriture, signatures, MSI, ARM64, benchmarks.

## Journal
- 2026-10-13 : audit PDFCraft, choix PDFium (D005), squelette → tranche verticale 1, environnement GNU faute de MSVC (D008),
  wgpu désactivé (D010), fixtures générées (`scripts/make_fixtures.py`), dépôt git initialisé.
