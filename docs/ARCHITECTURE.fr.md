# Architecture SuniPDF

[English](ARCHITECTURE.md)

## Principes

1. **Moteur d'abord.** Toute opération PDF vit dans `crates/sunipdf-core` (et, plus tard, dans des crates
   métier qui n'exposent que des types Rust). L'UI, la CLI, les tests et l'automatisation appellent la même API.
2. **L'UI ne touche jamais PDFium.** Elle n'embarque ni dépendance `pdfium-render`, ni logique de format.
3. **Un seul thread moteur.** PDFium n'est pas thread-safe : l'application confine `Engine` et les documents
   ouverts dans un thread de travail (`apps/sunipdf-desktop/src/worker.rs`) et dialogue par canaux `mpsc`.
   Le thread UI ne bloque jamais sur une opération PDF.
4. **Erreurs typées, textes traduits à la frontière.** `EngineError` porte un code stable (`code()`) ;
   seule l'UI mappe vers une clé i18n (`app.rs::error_key`), la CLI émet le code en JSON.
5. **Écritures atomiques.** Fichier temporaire voisin puis remplacement (`sunipdf_core::write_atomic`) pour
   les PDF, la récupération et les réglages. La validation indépendante qpdf reste une tâche QA distincte.
6. **Traitement PDF local, sans télémétrie.** OCR et signatures restent locaux. Recherche de mise à jour
   désactivable et téléchargement explicite contactent GitHub sans envoyer de PDF ; seul parcours réseau de l'application.
7. **Pas de JavaScript PDF.** Binaires PDFium compilés sans V8 ni XFA (`args.gn` : `pdf_enable_v8=false`).

## Workspace actuel

```
SuniPDF/
├── apps/
│   ├── sunipdf-desktop/      binaire `sunipdf` (egui/eframe) — main.rs, app.rs (état + document), shell.rs (chrome : onglets,
│   │                         barre d'outils, panneau « Tous les outils », accueil, rail, palette), workspace.rs (hub : table des
│   │                         outils → commandes du registre, outil actif, barre contextuelle), worker.rs, settings.rs, theme.rs, icons.rs,
│   │                         editor.rs (sélection, poignées, rotation, presse-papiers d'objets, barre d'objet), sign.rs
│   │                         (signature visuelle : pavé/saisie/import, bibliothèque locale, lots de pages ajustés, date), inline_text.rs
│   │                         (texte sur la page, sélecteur de polices), objects.rs (panneau Objets), pagesetup.rs (recadrer,
│   │                         redimensionner, page vierge, étiquettes), annot_adv.rs (encre, texte libre, formes, liens, sélection d'annotation, fils de réponses), view.rs (vue continue / page unique : mise en page virtualisée,
│   │                         tuiles voulues, pool de textures, zoom ancré)
│   └── sunipdf-cli/          binaire `sunipdf-cli` (clap) — sortie JSON, codes d'erreur stables
├── crates/
│   ├── sunipdf-core/         Engine, Document, RenderedPage, EngineError ; tiles.rs (géométrie des tuiles), cache.rs (LRU
│   │                         pages + tuiles) ; edit.rs (objets), geometry.rs (espace d'affichage), annot_write.rs / appearance.rs / links.rs
│   │                         (annotations avancées : lopdf écrit avec `/AP` explicite, PDFium rend et relit),
│   │                         clipboard.rs (copier/coller d'objets), fonts.rs (polices système), boxes.rs (cadres de page),
│   │                         signature_image.rs (signature visuelle : traits, fond retiré, PNG ; aperçu saisi PDFium natif et lots ajustés atomiques) ;
│   │                         page_labels.rs (plages bornées, préservation/réinitialisation, validation d'une copie) ;
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
| `sunipdf-forms` | non créé (D061) : AcroForm vit dans `sunipdf-core::{forms, form_data}` (modèle et écriture lopdf, apparences PDFium, JSON/XFDF et import FDF de valeurs) | — |
| `sunipdf-convert` | PDF↔images, texte, Office via convertisseur externe détecté | P4 |
| `sunipdf-ocr` | non créé : OCR hors ligne dans `sunipdf-core::ocr`, moteur et modèles embarqués | — |
| `sunipdf-security` | chiffrement (hors PDFium), permissions, caviardage, métadonnées | P5 |
| `sunipdf-signatures` | non créé : CMS/PKCS#12 dans `sunipdf-core::certificate` ; horodatage fiable encore prévu | — |
| `sunipdf-automation` | registre de commandes JSON-Schema, lots, MCP opt-in | P5 |
| `sunipdf-platform` | non créé : impression native dans `printing.rs` ; autres intégrations Windows encore prévues | — |
| `sunipdf-ui` | composants egui réutilisables (si l'app grossit) | P3 |

## Onglets (SUNI-012a)

Le moteur ne tient qu'un document actif. Un changement d'onglet envoie `Suspend` (le moteur renvoie
les octets du document, son mot de passe et son historique d'annulation) puis `Resume`/`ResumeDoc` (rechargement
depuis les octets via `Engine::open_bytes`). L'état d'interface du document (page, zoom, miniatures, recherche,
sélection, annotations, signets…) est échangé par `tabs.rs::take_tab` / `put_tab`. Coût : une sérialisation
PDFium par changement d'onglet ; mémoire : un instantané par onglet inactif. Le presse-papiers d'objets
(`ObjectClip`, un PDF d'une page en mémoire) vit dans l'état du moteur, hors document : il survit aux
changements d'onglet, ce qui permet de copier dans un document et de coller dans un autre.

## Parcours fonctionnels 0.9.0

La validation est suivie dans `PROJECT_STATUS.fr.md` ; cette section décrit la conception, pas un résultat de test.

- **Récupération** : `recovery.rs` écrit des PDF et métadonnées JSON atomiques par session dans
  `%LOCALAPPDATA%\SuniPDF\recovery`. Un verrou Windows exclusif écarte les sessions encore actives.
  Le worker prend un instantané du document modifié toutes les 60 secondes et à la suspension, sans remplacer
  la source. Le chiffrement du PDF est conservé, aucun mot de passe ne figure dans les métadonnées ; restauration par Enregistrer sous.
- **Impression** : `printing.rs` obtient un DC d'imprimante natif et une sélection de pages validée.
  Le thread moteur rend l'état courant par bandes tramées bornées, signale la progression par page et consulte
  un drapeau atomique d'annulation. Les handles sont possédés/libérés ; l'annulation n'enregistre pas le PDF.
- **Lecture et doubles pages** : `annot.rs` résout les points d'affichage en indices de caractères PDFium
  et renvoie texte/rectangles par page. Époque du document et génération de sélection accompagnent les messages ;
  l'UI garde une seule demande en attente et les dernières extrémités voulues. `view.rs::Layout` crée les lignes
  doubles, couverture seule et dernière page isolée, avec le même planificateur de 24 tuiles, pool de 192 Mio et cache moteur de 256 Mio.
- **Création de champs** : `forms.rs::add_form_field` valide nom et rectangle avant d'écrire AcroForm/Widget
  et de générer les apparences. `form_builder.rs` propose une boîte numérique ; `EditOp` permet l'annulation.
  La CLI `add-form-field` appelle la même méthode.
- **OCR** : `ocr.rs` lance uniquement Tesseract embarqué ou le moteur explicitement choisi, dans un dossier
  temporaire privé, avec modèles rapides `eng`/`fra`, sorties image/TSV bornées, délai maximal et annulation.
  Le travail se fait sur un document temporaire ; succès validé en une opération et un niveau d'annulation.
  Les pages déjà textuelles sont ignorées. Le texte invisible utilise la géométrie commune rotation/CropBox.
- **Certificats** : `certificate.rs` analyse les octets enregistrés et ajoute une révision CMS détachée
  incrémentale ; `certificate_native.rs` confie la clé privée à Windows Cryptography. Les buffers PFX/P12 et mots
  de passe sont effacés, sans import persistant de clé. La vérification décrit intégrité et certificat sans établir
  confiance, révocation ni horodatage fiable. UI `certificate.rs` et CLI `feature_cmd.rs` imposent une nouvelle copie
  pour signer et refusent les PDF chiffrés.

## Parcours des signatures visuelles 0.13.0 (publiée)

`Engine::typed_signature` crée un PDF temporaire, réutilise `Document::add_text` et ses contrôles de
police/glyphes existants, puis rend un PNG transparent borné via PDFium. `RenderSignature` s'exécute sur le
thread moteur ; l'identifiant de demande empêche un ancien aperçu de remplacer un texte, une police, une encre
ou une boîte modifiés. L'aperçu ne modifie pas le document ouvert. L'image obtenue emprunte le parcours existant
de bibliothèque PNG facultative et de placement.

`signature_image::placement_rect` fournit le fantôme de placement et les rectangles ajustés par page.
`Document::place_signature_image` valide et déduplique les cibles, modifie une copie temporaire via
`add_image_data`, consulte l'annulation entre les pages et avant la validation, puis remplace le document
uniquement en cas de succès. Le worker conserve un seul instantané d'annulation pour le lot validé ; un échec
préserve document et journal. La sélection Organiser est capturée à l'ouverture de la boîte ; un changement
structurel des pages abandonne la boîte ou les cibles capturées. CLI `add-image --pages` appelle la même méthode
de lot ; `--page` conserve son placement historique. Contrôles locaux, CI, paquets/publics et mises à niveau réussis ;
preuves exécutées et limites matérielles/interactives dans `PROJECT_STATUS.fr.md` (D102–D103).

## Parcours des étiquettes de page 0.14.0 (publiée)

`page_labels.rs` expose `PageLabelRange` et six valeurs `PageLabelStyle`. Le parseur/writer lopdf existant valide
l'arbre numérique avec bornes de profondeur/nœuds/plages/chaînes, conserve les règles hors plage physique modifiée
et rétablit la numérotation d'origine à la frontière suivante. Le writer protégé conserve chiffrement/mots de
passe/permissions. Un document candidat PDFium rafraîchit ses métadonnées avant de remplacer le document actif ;
un échec laisse l'état actif intact. Réinitialiser retire `PageLabels` du catalogue sans parcourir l'arbre malformé ;
les anciens objets devenus non référencés ne sont pas effacés de manière sécurisée (D104).

`pagesetup.rs` propose la boîte Pages/Organiser ; `EditOp` et le journal existant prennent en charge Appliquer et
Réinitialiser. CLI `set-page-labels`/`reset-page-labels` utilisent les mêmes API et garde de sortie.
`Snapshot::pages_unchanged` accompagne Annuler/Rétablir : vue/historique conservés pour les éditions
d'étiquettes/contenu, géométrie toujours invalidée par les opérations structurelles (D105). Les étiquettes restent
positionnelles après ces opérations. Contrôles locaux, CI, paquets/publics, mises à niveau et mise à jour de production
réussis ; preuves exécutées et limites restantes dans `PROJECT_STATUS.fr.md`.

## Destinations des liens existants 0.15.0 (publiée)

`Document::set_link_target` réutilise le writer d'annotations et la réécriture protégée. Il vérifie le type Link,
résout les drapeaux `F` directs/indirects et refuse ReadOnly/Locked/LockedContents ou les valeurs malformées.
L'écriture partagée accepte une URI autorisée ou une page physique `/Fit`, remplace ensemble `A`/`Dest`/`AA` et
conserve le reste du dictionnaire, la géométrie et les données indépendantes du document (D106).

`annot_adv.rs` réutilise la boîte de lien pour Modifier la destination/double-clic sur une annotation sélectionnée.
Les destinations résolues par PDFium préremplissent la page ; une cible inconnue donne une adresse vide. La boîte
précise que le remplacement affiche la page entière (Fit). Éditions/changements d'onglet abandonnent les boîtes
périmées ; le worker applique une édition ordinaire annulable, sans changer la géométrie. CLI `set-link-target`
utilise les mêmes API et garde de sortie. Publié en 0.15.0 ; contrôles locaux, CI, mises à niveau, paquets préparés/publics
et metteur à jour réussis ; preuves exécutées et limites restantes dans `PROJECT_STATUS.fr.md`.
Confirmation des adresses externes et avertissement de réécriture des documents signés inchangés.

## Bordures en tirets des formes 0.16.0 (en développement)

`AnnotStyle::dash_pattern` et les détails d'annotation transportent le vecteur réel plutôt qu'un booléen.
Rectangle, ellipse et ligne/flèche écrivent un style/tableau `BS` cohérent avec les tirets de l'état graphique
`/AP`, phase zéro ; un motif vide conserve le parcours continu existant. Remplissage et pointes de flèche restent
pleins (D107). Les dictionnaires existants sont résolus avant de régénérer une apparence SuniPDF ; un style
malformé/non pris en charge refuse l'édition au lieu de perdre le motif. Valeurs et travail estimé sont bornés.
`D []` explicite est un continu valide ; `D` absent devient `[3]`. Une bordure SuniPDF invalide désactive
Déplacer/Redimensionner/Style ; le commentaire de forme garde `/AP` intact, celui d'un texte libre la régénère
et refuse donc une bordure malformée. Aperçu borné au même seuil de 100 000, motifs valides sous-pixel conservés ;
un motif non vide invalide/excessif omet le contour au lieu d'un repli continu.

`annot_adv.rs` conserve le vecteur chargé lors de la sélection et des éditions ordinaires ; un changement réel
de la case choisit `[6, 3]` ou continu. CLI `annotate --dashed` fournit le même motif fixe ; `annotations` relit
`dash_pattern`. Motif non vide sur un type non pris en charge : erreur. Annulation/protection/gardes de sortie
existantes réutilisées, sans nouvelle dépendance. Validation en cours dans `PROJECT_STATUS.fr.md`.

## Coordonnées (SUNI-031c)

Toute coordonnée qui franchit la frontière moteur ↔ interface/CLI est en **espace d'affichage** : points,
origine au coin bas-gauche de la page telle qu'affichée (après `/Rotate`, boîte MediaBox ∩ CropBox), Y vers
le haut — l'espace de `Document::page_size` et du rendu. `sunipdf-core::geometry::PageGeometry` convertit
vers l'espace utilisateur PDF (objets, texte, annotations) ; c'est le seul endroit qui connaît `/Rotate` et
les boîtes. L'UI convertit seulement écran ↔ affichage (`annotate::to_pdf`). Les tests
`geometry_matches_rendered_pixels_on_rotated_and_cropped_pages` et suivants vérifient la convention au pixel.

## Registre de commandes (SUNI-010, livré)

Les modifications annulables s'exécutent sur le thread moteur. Les éditions ordinaires utilisent
`worker.rs::State::edit` ; les tâches atomiques, dont l'OCR et les signatures sur plusieurs pages, ajoutent un
seul instantané antérieur après validation réussie. `Document::snapshot` fournit les octets PDF ; `Undo`/`Redo` rechargent l'instantané
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
