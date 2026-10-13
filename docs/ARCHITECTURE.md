# Architecture SuniPDF

## Principes

1. **Moteur d'abord.** Toute opération PDF vit dans `crates/sunipdf-core` (et, plus tard, dans des crates
   métier qui n'exposent que des types Rust). L'UI, la CLI, les tests et l'automatisation appellent la même API.
2. **L'UI ne touche jamais PDFium.** Elle n'embarque ni dépendance `pdfium-render`, ni logique de format.
3. **Un seul thread moteur.** PDFium n'est pas thread-safe : l'application confine `Engine` et les documents
   ouverts dans un thread de travail (`apps/sunipdf-desktop/src/worker.rs`) et dialogue par canaux `mpsc`.
   Le thread UI ne bloque jamais sur une opération PDF.
4. **Erreurs typées, textes traduits à la frontière.** `EngineError` porte un code stable (`code()`) ;
   seule l'UI mappe vers une clé i18n (`app.rs::error_key`), la CLI émet le code en JSON.
5. **Écritures atomiques.** Fichier temporaire voisin puis `rename` (`settings.rs::save_atomic`) ; même règle
   pour les futurs enregistrements de PDF, avec vérification par un outil indépendant (qpdf) en CI.
6. **Hors ligne, sans télémétrie.** Aucune dépendance réseau à l'exécution. Le script de téléchargement
   de PDFium est un outil de build, pas une fonction de l'application.
7. **Pas de JavaScript PDF.** Binaires PDFium compilés sans V8 ni XFA (`args.gn` : `pdf_enable_v8=false`).

## Workspace actuel

```
SuniPDF/
├── apps/
│   ├── sunipdf-desktop/      binaire `sunipdf` (egui/eframe) — main.rs, app.rs, worker.rs, settings.rs, theme.rs
│   └── sunipdf-cli/          binaire `sunipdf-cli` (clap) — sortie JSON, codes d'erreur stables
├── crates/
│   ├── sunipdf-core/         Engine, Document, RenderedPage, EngineError ; tests/fixtures/*.pdf
│   └── sunipdf-i18n/         Language, tr(), tr_args(), catalogues embarqués
├── assets/i18n/              fr-FR.json, en-US.json (source de vérité des textes)
├── third_party/pdfium/       binaires PDFium (ignorés par git, scripts/fetch-pdfium.ps1)
├── scripts/                  dev-env.ps1, fetch-pdfium.ps1, start.ps1, check-i18n.ps1, make_fixtures.py
├── docs/                     cette documentation
└── .claude/agents/           12 sous-agents
```

## Flux « ouvrir → afficher → naviguer → fermer »

```
UI (thread principal)                     Worker (thread "sunipdf-engine")
---------------------                     --------------------------------
Request::Open{path,pw}  ───────────────▶  Engine::open → Response::Opened(DocumentInfo)
                                          (ou OpenFailed{PasswordRequired…} → boîte de mot de passe)
calcule zoom effectif, échelle px/pt
Request::Render{page,scale,gen} ───────▶  Document::render_page → Response::Rendered{gen, RGBA}
(un seul rendu en vol ; gen obsolète ignorée)
texture egui ← RGBA ; affichage à la taille logique (net à tout DPI)
Request::Close ────────────────────────▶  drop(Document)
```

Le zoom « ajuster à la largeur/page » est recalculé à chaque frame ; l'échelle demandée est quantifiée
(`quantize`) pour éviter un rendu par pixel de redimensionnement. Le rendu est plafonné à
`MAX_RENDER_SIDE_PX` (16 384 px de côté) pour borner la mémoire.

## Crates planifiés (créés seulement quand une fonctionnalité réelle les justifie)

| Crate | Rôle | Phase |
|---|---|---|
| `sunipdf-render` | cache LRU plafonné, tuiles, pré-rendu des pages visibles, miniatures | P2 |
| `sunipdf-pages` | fusion, division, extraction, rotation, réorganisation (PDFium `FPDF_ImportPages`, `FPDFPage_*`) | P2 |
| `sunipdf-editor` | annotations, texte, images, filigranes | P2–P3 |
| `sunipdf-forms` | AcroForm (FPDF_FORMFILL), FDF/XFDF | P3 |
| `sunipdf-convert` | PDF↔images, texte, Office via convertisseur externe détecté | P4 |
| `sunipdf-ocr` | Tesseract (fra, eng) hors ligne | P4 |
| `sunipdf-security` | chiffrement (hors PDFium), permissions, caviardage, métadonnées | P5 |
| `sunipdf-signatures` | CMS, PKCS#12, horodatage, vérification | P5 |
| `sunipdf-automation` | registre de commandes JSON-Schema, lots, MCP opt-in | P5 |
| `sunipdf-platform` | Explorateur Windows, associations, impression, DPI | P5 |
| `sunipdf-ui` | composants egui réutilisables (si l'app grossit) | P3 |

## Registre de commandes (P2)

Un enum `Command` + `CommandRegistry` dans `sunipdf-core` décrira chaque opération (identifiant stable,
paramètres sérialisables, précondition). UI, CLI, palette de commandes, lots et MCP l'exploiteront ;
l'historique undo/redo s'appuiera sur ce même registre.

## Choix d'infrastructure

| Besoin | Choix | Raison |
|---|---|---|
| Rendu / parsing | PDFium 7881 via `pdfium-render` 0.9.4, DLL chargée à l'exécution | Fidélité de Chrome, ARM64 disponible, pas de compilation C++ |
| UI | `eframe`/`egui` 0.36 | Rust natif, immédiat, DPI géré, accessibilité AccessKit intégrée |
| Préférences | JSON dans `%APPDATA%\SuniPDF` | Zéro dépendance ; SQLite seulement si des métadonnées volumineuses apparaissent |
| Erreurs | `thiserror` | Types explicites, codes stables |
| Journalisation | `log` + `env_logger` | Suffisant ; `tracing` si des spans deviennent nécessaires |
| Tests | `cargo test`, fixtures générées par `scripts/make_fixtures.py` | Reproductible sans dépendance Python externe |

## Sécurité

- Validation des chemins par l'OS ; aucune exécution de contenu PDF ; limites de taille de rendu.
- Les documents ne quittent jamais la machine.
- Fichiers hostiles : PDFium est fuzzé en continu par Chromium ; SuniPDF ajoutera son propre corpus (P6).
