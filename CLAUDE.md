# SuniPDF — règles permanentes Claude Code

## Projet
Application **desktop native Windows 10/11** de gestion de PDF, hors ligne, sans télémétrie. Workspace Rust,
UI egui/eframe, moteur **PDFium** (`pdfium-render`, DLL chargée à l'exécution). Référence fonctionnelle :
https://github.com/storytold/pdfcraft — auditée dans `docs/PDFCRAFT_AUDIT.md` ; aucune copie de code ni de
ressource sans passer par `docs/THIRD_PARTY_NOTICES.md`.

## Langues, impératif
- UI totalement traduisible en `fr-FR` (défaut) et `en-US` ; sélecteur dans Paramètres ; changement à chaud ;
  préférence persistée (`%APPDATA%\SuniPDF\settings.json`).
- Aucune chaîne UI littérale dans `apps/` : `sunipdf_i18n::tr` / `tr_args`. Inclure menus, erreurs, tooltips,
  dialogues, raccourcis affichés, accessibilité (`a11y.*`).
- Toute nouvelle clé va dans `assets/i18n/fr-FR.json` **et** `en-US.json` ; tests `cargo test -p sunipdf-i18n -p sunipdf-desktop`.
- Ne jamais traduire le contenu des PDF. La CLI parle anglais + JSON (D009).
- Me répondre en français.

## Architecture (détails : docs/ARCHITECTURE.md)
- Moteur (`crates/sunipdf-core`) découplé de l'UI et de la CLI ; l'UI n'importe jamais `pdfium-render`.
- PDFium n'est pas thread-safe : un seul thread moteur (`apps/sunipdf-desktop/src/worker.rs`).
- Pas de parseur PDF maison. Tâches lourdes hors thread UI. Fichiers jamais envoyés sur Internet.
- Erreurs typées (`EngineError`, `code()` stable) ; l'UI mappe vers des clés `error.*`.
- Sauvegardes atomiques (tmp + rename), aucun écrasement silencieux, vérification qpdf dès P2.
- Pas de bouton factice ; pas de JavaScript PDF (binaires sans V8/XFA).
- Fonctions non implémentées → roadmap, jamais « terminé ».

## Commandes
```powershell
. .\scripts\dev-env.ps1          # PATH (MSVC si présent, sinon MinGW WinLibs)
.\scripts\fetch-pdfium.ps1       # third_party/pdfium/win-x64/bin/pdfium.dll (ignoré par git)
cargo test --workspace -j 2      # -j 2 : poste à 14 Go (docs/KNOWN_ISSUES.md)
cargo run -p sunipdf-desktop -- chemin\vers\fichier.pdf
cargo run -p sunipdf-cli -- info fichier.pdf
cargo run -p sunipdf-cli -- render fichier.pdf --page 1 --scale 2 --out page1.png
.\scripts\check-i18n.ps1
python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures   # régénérer les PDF de test
python -I scripts\make_icon.py assets\icon                          # régénérer l'icône
.\scripts\package-portable.ps1 -Arch x64                          # ZIP portable + SHA256 (release)
.\scripts\package-msi.ps1 -Arch x64                               # MSI (WiX v3 requis)
```
Cible MSVC active (Build Tools installés). Repli GNU + WinLibs documenté dans `scripts/dev-env.ps1` si un poste n'a pas de droits admin.

## Modules existants
`crates/sunipdf-core` (Engine, Document, render, text, pages, cache, command), `crates/sunipdf-i18n`,
`apps/sunipdf-desktop` (sunipdf.exe : app.rs état/document, shell.rs chrome, worker.rs thread moteur, icons.rs polices système),
`apps/sunipdf-cli`. Vérification visuelle : `.\scripts\dev-screenshot.ps1 -Out capture.png [-Pdf x.pdf] [-Keys "^k"]`. Crates futurs : voir `docs/ARCHITECTURE.md`.

## Orchestration
12 agents dans `.claude/agents/` (stratégie : `docs/AGENT_STRATEGY.md`). Déléguer selon expertise, dépendances
et coût ; jamais tous en parallèle sans raison ; worktrees si écritures concurrentes. L'agent principal intègre,
vérifie, exécute les tests et met à jour l'état. Ne pas prétendre avoir lancé un agent s'il n'a pas été appelé.
Chaque agent lit uniquement `CLAUDE.md`, `docs/ARCHITECTURE.md` et son périmètre.

## Procédure
1. Lire `docs/PROJECT_STATUS.md` et `docs/NEXT_TASKS.md`.
2. Proposer brièvement la tranche verticale suivante.
3. Implémenter, compiler, tester ; rapporter commandes et résultats réels (jamais inventés).
4. Mettre à jour `PROJECT_STATUS.md`, `NEXT_TASKS.md`, `DECISIONS.md`, `KNOWN_ISSUES.md` si nécessaire.
5. Ne jamais annoncer MSI / ARM64 / test Windows sans résultat réel.

## Critères de validation d'une fonctionnalité
Code réel + commande connectée + tests automatisés + test d'intégration + gestion d'erreurs + fichier de sortie
vérifié (qpdf/relecture) + état documenté + aucune régression (`cargo test --workspace`, clippy `-D warnings`, fmt).
