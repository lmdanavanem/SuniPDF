# Stratégie multi-agents

L'orchestrateur (session Claude Code principale) garde la responsabilité des contrats, de l'intégration,
des tests et de l'état du projet. Les 12 sous-agents sont définis dans `.claude/agents/*.md` (identité,
périmètre, chemins autorisés, tâches, critères d'acceptation, tests, protocole de rapport, budget de contexte).

| Agent | Fichier | Modèle | Périmètre principal |
|---|---|---|---|
| 01 ARCHITECT | architect.md | opus | docs/, Cargo.toml, contrats inter-crates, ADR |
| 02 PDF-ENGINE | pdf-engine.md | sonnet | crates/sunipdf-core |
| 03 RENDERING | rendering.md | sonnet | crates/sunipdf-render |
| 04 UI-UX | ui-ux.md | sonnet | apps/sunipdf-desktop, crates/sunipdf-i18n, assets/i18n |
| 05 PDF-EDITOR | pdf-editor.md | sonnet | crates/sunipdf-editor |
| 06 PAGE-MANAGER | page-manager.md | sonnet | crates/sunipdf-pages |
| 07 OCR-CONVERSION | ocr-conversion.md | sonnet | crates/sunipdf-ocr, sunipdf-convert |
| 08 SECURITY-SIGNATURES | security-signatures.md | opus | crates/sunipdf-security, sunipdf-signatures |
| 09 FORMS-ACCESSIBILITY | forms-accessibility.md | sonnet | crates/sunipdf-forms |
| 10 AUTOMATION | automation.md | sonnet | apps/sunipdf-cli, crates/sunipdf-automation, sunipdf-platform |
| 11 QA-PERFORMANCE | qa-performance.md | opus | tests/, benches/, corpus, docs/QA_* (jamais le code fonctionnel) |
| 12 WINDOWS-RELEASE | windows-release.md | sonnet | packaging/, .github/workflows, scripts/, assets/ (hors i18n) |

## Graphe de dépendances

```
ARCHITECT ─▶ PDF-ENGINE ─▶ { RENDERING, PAGE-MANAGER, PDF-EDITOR, FORMS, SECURITY, OCR-CONVERSION } ─▶ UI-UX
                                              │                                                       │
                                              └──────────────▶ AUTOMATION ◀───────────────────────────┘
QA-PERFORMANCE : après chaque livraison (indépendant)      WINDOWS-RELEASE : P1 (CI), P6 (MSI/ARM64)
```

## Règles d'invocation

1. L'architecte définit un contrat (types, erreurs, clés i18n) avant tout développement parallèle.
2. Parallélisme seulement entre crates disjoints ; jamais deux agents sur le même fichier.
   Git worktrees (`EnterWorktree`) quand plusieurs agents modifient du code simultanément.
3. Un agent reçoit uniquement : `CLAUDE.md`, `docs/ARCHITECTURE.md`, les fichiers de son périmètre,
   et le contrat de la tâche. Pas de chargement du dépôt entier.
4. Modèle : `opus` pour architecture, sécurité, QA ; `sonnet` pour les tâches cadrées. Si un modèle
   n'est pas exposé par l'environnement, l'orchestrateur le dit et n'invente pas.
5. Format de tâche :

```yaml
task_id: SUNI-001
owner: PDF-ENGINE
objective: Description concrète
dependencies: []
allowed_paths: []
acceptance_criteria: []
tests_required: []
expected_outputs: []
status: pending
```

6. Après chaque livraison : relecture du diff, vérification du contrat, exécution des tests concernés,
   intégration, résolution des conflits, mise à jour de `PROJECT_STATUS.md` et `NEXT_TASKS.md`.
7. Rapport d'agent obligatoire : fichiers modifiés / commandes exécutées et résultats réels /
   limites non vérifiées / nouvelles clés i18n (fr + en). Aucun résultat de test inventé.

## Journal des invocations

| Date | Agent | Tâche | Résultat |
|---|---|---|---|
| 2026-10-13 | architect | Réécriture des 12 définitions d'agents (SUNI-000) | 12 fichiers, 58–62 lignes, 8 sections chacun ; incohérences signalées et traitées dans `DECISIONS.md` |
