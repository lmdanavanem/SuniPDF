# Audit de PDFCraft (storytold/pdfcraft)

Date de l'audit : 2026-10-13. Sources consultées : page GitHub, README, `Cargo.toml` racine, `NOTICE`,
`ATTRIBUTION.md`, `ROADMAP.md`, arborescence git (API GitHub, commit `b5467d9`). Le code n'a pas été
cloné ni compilé localement : tout ce qui suit relève de la lecture de la documentation et des manifestes.

Légende : **[V]** fait vérifié dans les sources citées · **[H]** hypothèse raisonnable · **[F]** possibilité future.

## 1. Identité

| Élément | Valeur |
|---|---|
| Dépôt | https://github.com/storytold/pdfcraft, créé le 2026-09-30, actif (push le 2026-10-13) **[V]** |
| Positionnement | « The PDF workbench ; clean-room reimplementation of Adobe Acrobat, rebuilt in pure Rust », famille ArtCraft **[V]** |
| Statut | **alpha** (déclaré) ; ≈ 12,5 k étoiles, 4,8 k forks, 536 issues ouvertes, 307 PR ouvertes **[V]** |
| Version | 0.7.1, édition Rust 2024, `rust-version = 1.95` **[V]** |
| Plateformes | macOS, Windows, Linux, FreeBSD, Web (WebAssembly) **[V]** |
| Licence code | `MIT OR Apache-2.0` (fichiers LICENSE-MIT, LICENSE-APACHE, NOTICE) **[V]** |
| Marques | Nom, wordmark et logos ArtCraft : marques déposées, **non libres**, à retirer de tout fork modifié **[V]** |

## 2. Architecture et stack

- Workspace Cargo d'environ 30 crates sous `crates/` + `apps/` (desktop `pdfcraft`, `pdfcraft-cli`, `pdfcraft-web`) + `xtask` **[V]**.
- Principe « engine first » : `pdfcraft-engine` est l'unique façade ; GUI, CLI, MCP et API Rust sont des frontends **[V]**.
- Rendu des pages délégué au crate tiers **hayro 0.7.1** (vendored et patché), avec `hayro-syntax`, `lopdf 0.45` pour les objets **[V]**. Un moteur de rendu maison est en cours **[V]**.
- UI : `egui`/`eframe` 0.36, `egui_kittest` pour les tests UI, `rfd` 0.17 **[V]**. Dépendances git `craft-ui`, `craft-print` (storytold/craft-libs) **[V]**.
- Interdit `unsafe_code` au niveau workspace ; Clippy `-D warnings` ; 200+ tests ; règle de stratification des crates ; audit des licences d'assets **[V]**.
- Crates visibles : filters, crypt, cos, organize, fonts, annot, forms, render, engine, automation, ui-egui, a11y, audit, compare, content, create, edit, export (pptx/zip), geom, js (FormCalc/XFA), measure, model, ocr, optimize, platform, preflight, print, redact, sign (CMS, PKCS#12, timestamp, Windows keychain), xfa, xfdf **[V]**.

## 3. Fonctionnalités déclarées

Voir la matrice détaillée dans `FEATURE_MATRIX.md`. En résumé **[V]** :
lecture complète (typographie avancée, tuiles, modes de vue, thèmes), recherche et sélection en ordre de lecture,
organisation de pages avec undo profond, fusion/division/extraction préservant liens, champs, calques et pièces
jointes, sauvegarde incrémentale atomique vérifiée par qpdf, autosave et récupération, chiffrement standard
complet (RC4 40 bits → AES-256), formulaires (AcroForm, détection XFA), commentaires, calques, pièces jointes,
palette de commandes, CLI JSON, serveur MCP opt-in, canal de contrôle de l'app, 22 langues d'interface via `.tsv`.

## 4. Limites documentées par le projet lui-même **[V]**

- ≈ 57 % des fonctions hors-ligne d'Acrobat Pro présentes « sous une forme ou une autre » ; ≈ 55 % pour un usage quotidien.
- Conversion : pas d'Office → PDF ; export Word perd la mise en page ; pas d'Excel/PowerPoint.
- Édition de texte existant « mince », surtout CJK et bidi.
- OCR limité au latin ; modèles `ocrs` téléchargés à part (CC-BY-SA-4.0 → obligations d'attribution et partage à l'identique).
- Pas de jetons de signature (PKCS#11), pas de LTV en ligne, pas de listes de confiance AATL/EUTL.
- Preflight et PDF/A/X/UA absents ; XFA complet absent ; impression web absente.
- Le fuzzing trouve encore des plantages et des blocages sur fichiers hostiles.
- Windows : barre de titre système, erreur DX12 x86 non résolue, hinting basse résolution mesuré sur macOS seulement.
- Corpus pdf.js : 963/983 fichiers s'ouvrent et se rendent sans plantage ; 958 survivent à ouvrir-éditer-enregistrer.

## 5. Licences et obligations

| Composant | Licence | Obligation pour SuniPDF |
|---|---|---|
| Code PDFCraft | MIT OR Apache-2.0 | Conserver copyright + NOTICE si réutilisation ; marquer les modifications (Apache) **[V]** |
| hayro / hayro-interpret | Apache-2.0 OR MIT | Idem ; polices Foxit std-14 (BSD-3) et CMaps Adobe (BSD-3) embarquées **[V]** |
| lopdf | MIT | Conserver la notice **[V]** |
| Icônes Lucide | ISC | Conserver la notice si réutilisées **[V]** |
| Polices Inter, JetBrains Mono, Dancing Script | OFL-1.1 | Conserver OFL ; pas de vente isolée **[V]** |
| Modèles OCR ocrs | CC-BY-SA-4.0 | Attribution + partage à l'identique des dérivés : **à éviter** pour un produit commercial **[V]** |
| Marques ArtCraft, logos, captures avec logo | Propriétaire | **Ne jamais réutiliser** **[V]** |

## 6. Points forts à retenir

1. Séparation moteur / frontends et registre d'outils JSON-Schema exploité par la CLI et le MCP **[V]**.
2. Discipline de sauvegarde : incrémentale, atomique, vérifiée par un outil indépendant (qpdf) **[V]**.
3. Autosave + récupération après plantage **[V]**.
4. Corpus de non-régression large (pdf.js) et chiffres publiés **[V]**.
5. Sécurité : pas de réseau, MCP confiné à `--root`, canal de contrôle loopback + jeton **[V]**.

## 7. Décision pour SuniPDF (voir `DECISIONS.md` D005–D008)

- **Ne pas forker PDFCraft** : stack jeune (crates internes 0.7.x, dépendances git épinglées, hayro vendored/patché),
  rendu encore délégué à un crate expérimental, marque à retirer, et surface énorme à maintenir **[H]**.
- **Reprendre les idées d'architecture** (façade moteur, registre de commandes, sauvegarde atomique vérifiée, corpus)
  sans copier de code dans un premier temps. Toute réutilisation ultérieure de code MIT/Apache passera par
  `docs/THIRD_PARTY_NOTICES.md` avec notice conservée **[V]**.
- **Moteur de rendu : PDFium** (bblanchon/pdfium-binaries, BSD-3-Clause ; wrapper `pdfium-render` MIT/Apache) :
  moteur de Chrome, éprouvé sur des milliards de documents, binaires x64 **et ARM64** Windows disponibles,
  rendu + texte + formulaires + annotations + import de pages dans une seule API **[V]**.
- hayro (0.8.0 publié le 2026-10-04, Apache/MIT) reste le candidat « pur Rust » de repli ; à réévaluer en phase 6 **[F]**.
