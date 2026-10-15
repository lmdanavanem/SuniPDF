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
│   ├── sunipdf-desktop/      binaire `sunipdf` (egui/eframe) — main.rs, app.rs (état + document), shell.rs (chrome : onglets,
│   │                         barre d'outils, panneau « Tous les outils », accueil, rail, palette), workspace.rs (hub : table des
│   │                         outils → commandes du registre, outil actif, barre contextuelle), worker.rs, settings.rs, theme.rs, icons.rs,
│   │                         editor.rs (sélection, poignées, rotation, presse-papiers d'objets, barre d'objet), sign.rs
│   │                         (signature visuelle : pavé, import, bibliothèque locale, placement, date), inline_text.rs
│   │                         (texte sur la page, sélecteur de polices), objects.rs (panneau Objets), pagesetup.rs (recadrer,
│   │                         redimensionner, page vierge), view.rs (vue continue / page unique : mise en page virtualisée,
│   │                         tuiles voulues, pool de textures, zoom ancré)
│   └── sunipdf-cli/          binaire `sunipdf-cli` (clap) — sortie JSON, codes d'erreur stables
├── crates/
│   ├── sunipdf-core/         Engine, Document, RenderedPage, EngineError ; tiles.rs (géométrie des tuiles), cache.rs (LRU
│   │                         pages + tuiles) ; edit.rs (objets), geometry.rs (espace d'affichage),
│   │                         clipboard.rs (copier/coller d'objets), fonts.rs (polices système), boxes.rs (cadres de page),
│   │                         signature_image.rs (signature visuelle : traits tramés, fond retiré, rognage, PNG ; sans PDFium) ;
│   │                         tests/fixtures/*.pdf (dont boxes.pdf : pages pivotées et rognées)
│   └── sunipdf-i18n/         Language, tr(), tr_args(), catalogues embarqués
├── assets/i18n/              fr-FR.json, en-US.json (source de vérité des textes)
├── third_party/pdfium/       binaires PDFium (ignorés par git, scripts/fetch-pdfium.ps1)
├── scripts/                  dev-env.ps1, fetch-pdfium.ps1, start.ps1, check-i18n.ps1, make_fixtures.py, dev-screenshot.ps1 (capture de la fenêtre)
├── docs/                     cette documentation
```

## Flux « ouvrir → afficher → naviguer → fermer »

```
UI (thread principal)                     Worker (thread "sunipdf-engine")
---------------------                     --------------------------------
Request::Open{path,pw}  ───────────────▶  Engine::open → Response::Opened(DocumentInfo)
                                          (ou OpenFailed{PasswordRequired…} → boîte de mot de passe)
calcule zoom, mise en page, zone visible
Request::SetWanted{epoch, [TileJob]} ──▶  remplace la file des tuiles voulues (rien n'est rendu tout de suite)
                                          entre deux requêtes, UNE tuile : cache LRU ou
                                          Document::render_region → Response::Tile{epoch, job, RGBA}
texture egui ← RGBA (une tuile par réponse) ; époque différente → ignorée
Request::Close ────────────────────────▶  drop(Document)
```

## Vue continue et tuiles (SUNI-012b, D038)

- **Fenêtre de pages** : en défilement continu tout le document, en page unique la page courante ; une seule
  machinerie (`view.rs` : `Layout`, `paint_slot`, `handle_page_event`) sert les deux modes. `Layout` place les
  pages (marge 16, intervalle 12) ; seules celles qui coupent la zone visible créent widgets, demandes et
  textures (recherche dichotomique), plus la page d'un glissement ou de l'éditeur de texte en cours.
- **Tuiles** (`sunipdf-core::tiles`) : page entière jusqu'à 4096 px et 8 Mpx, sinon grille de 1024 px ancrée en
  (0, 0), bornée par `max_texture_side` du GPU ; `Document::render_region` rend une zone sans allouer la page
  (origine négative de PDFium), jusqu'à 131 072 px de côté de page. Échelle quantifiée au 1/200 (`quantize_milli`).
- **Annulation par remplacement** : l'UI envoie la liste des tuiles voulues (visibles du centre vers les bords,
  puis le bord des pages voisines, 24 au plus) seulement quand elle change ; une tuile commencée va au bout.
  L'époque change à chaque ouverture, onglet, édition : les réponses d'une autre époque sont jetées.
- **Pool de textures** (`TexturePool`, 192 Mio, LRU, jamais une texture de l'image courante) ; pendant un zoom
  ou après une retouche (`DocState.pages_unchanged`), les textures d'une autre échelle ou périmées restent
  affichées, étirées, jusqu'aux nouvelles tuiles ; le cache moteur (256 Mio) sert tuiles et miniatures.
- **Défilement** : requêtes (`ScrollRequest` : page, point d'une page, décalage, relatif) résolues à l'image
  suivante ; la page courante suit la page dominante, sauf verrou après un « aller à » ; le zoom garde le point
  ancré (page, fraction) sous le pointeur ou au centre.
`render_page` (export d'images, CLI sans `--region`) reste plafonné à `MAX_RENDER_SIDE_PX` (16 384 px).

## Crates planifiés (créés seulement quand une fonctionnalité réelle les justifie)

| Crate | Rôle | Phase |
|---|---|---|
| `sunipdf-render` | non créé (D038) : tuiles et cache vivent dans `sunipdf-core::{tiles, cache}` ; à extraire si un second consommateur apparaît | — |
| `sunipdf-pages` | fusion, division, extraction, rotation, réorganisation (PDFium `FPDF_ImportPages`, `FPDFPage_*`) | P2 |
| `sunipdf-editor` | annotations, texte, images, filigranes | P2–P3 |
| `sunipdf-forms` | non créé (D061) : AcroForm vit dans `sunipdf-core::{forms, form_data}` (modèle et écriture lopdf, apparences PDFium, JSON) ; FDF/XFDF non livrés | — |
| `sunipdf-convert` | PDF↔images, texte, Office via convertisseur externe détecté | P4 |
| `sunipdf-ocr` | Tesseract (fra, eng) hors ligne | P4 |
| `sunipdf-security` | chiffrement (hors PDFium), permissions, caviardage, métadonnées | P5 |
| `sunipdf-signatures` | CMS, PKCS#12, horodatage, vérification | P5 |
| `sunipdf-automation` | registre de commandes JSON-Schema, lots, MCP opt-in | P5 |
| `sunipdf-platform` | Explorateur Windows, associations, impression, DPI | P5 |
| `sunipdf-ui` | composants egui réutilisables (si l'app grossit) | P3 |

## Onglets (SUNI-012a)

Le moteur ne tient qu'un document actif. Un changement d'onglet envoie `Suspend` (le moteur renvoie
les octets du document, son mot de passe et son historique d'annulation) puis `Resume`/`ResumeDoc` (rechargement
depuis les octets via `Engine::open_bytes`). L'état d'interface du document (page, zoom, miniatures, recherche,
sélection, annotations, signets…) est échangé par `tabs.rs::take_tab` / `put_tab`. Coût : une sérialisation
PDFium par changement d'onglet ; mémoire : un instantané par onglet inactif. Le presse-papiers d'objets
(`ObjectClip`, un PDF d'une page en mémoire) vit dans l'état du moteur, hors document : il survit aux
changements d'onglet, ce qui permet de copier dans un document et de coller dans un autre.

## Coordonnées (SUNI-031c)

Toute coordonnée qui franchit la frontière moteur ↔ interface/CLI est en **espace d'affichage** : points,
origine au coin bas-gauche de la page telle qu'affichée (après `/Rotate`, boîte MediaBox ∩ CropBox), Y vers
le haut — l'espace de `Document::page_size` et du rendu. `sunipdf-core::geometry::PageGeometry` convertit
vers l'espace utilisateur PDF (objets, texte, annotations) ; c'est le seul endroit qui connaît `/Rotate` et
les boîtes. L'UI convertit seulement écran ↔ affichage (`annotate::to_pdf`). Les tests
`geometry_matches_rendered_pixels_on_rotated_and_cropped_pages` et suivants vérifient la convention au pixel.

## Registre de commandes (SUNI-010, livré)

Annulation : chaque modification de pages passe par `worker.rs::State::edit`, qui prend un instantané
(`Document::snapshot`, octets PDF) avant d'appliquer l'opération ; `Undo`/`Redo` rechargent l'instantané
(`Document::restore`). Journal plafonné (50 entrées, 512 Mo). Simple, générique, borné en mémoire ;
un journal de commandes nommées viendra si la mémoire devient contraignante.

`crates/sunipdf-core/src/command.rs` : `Command` (enum), `REGISTRY` (tableau constant : identifiant stable,
clé i18n du libellé, clé i18n du raccourci, `needs_document`, menu). Données pures, sans dépendance UI.
- L'application construit ses menus Fichier / Affichage / Aide depuis le registre et route menus **et**
  raccourcis clavier vers un seul point d'entrée `SuniApp::execute(ctx, Command)`.
- La CLI expose `sunipdf-cli commands` (JSON). La palette de commandes, les raccourcis configurables,
  l'undo/redo et l'automatisation s'appuieront sur ces mêmes identifiants.
- Tests : unicité des identifiants, aller-retour `from_id`, existence des clés i18n dans les deux catalogues.

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
