# Plan d'implémentation — tranche `bookmarks-links` (signets, liens, étiquettes de pages, cohérence de structure)

Statut : plan (aucune ligne de code écrite). Rédigé le 2026-10-14 à partir de la lecture intégrale de
`pages.rs`, `security.rs`, `annot.rs`, `lib.rs`, `error.rs`, `command.rs`, `worker.rs`, `app.rs` (réponses, entrées,
`execute`, `document_area`), `shell.rs`, `dialog.rs`, `docinfo.rs`, `annotate.rs`, `editor.rs`, `tabs.rs`, `tools.rs`
(début), `icons.rs`, de la CLI, des tests et de `make_fixtures.py`, puis des sources **réelles** des bibliothèques
(chemins ci-dessous). Rien n'a été compilé ni exécuté (contrainte de la tâche) : tout comportement de PDFium non lisible
dans les sources Rust est marqué **[H#]** (hypothèse) et soumis à l'étape 0 (section 10).

Abréviations de chemins pour les citations :
- `PR` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src`
- `LP` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lopdf-0.45.0/src`
- `EG` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.2/src`
- `CORE` = `crates/sunipdf-core/src`, `DESK` = `apps/sunipdf-desktop/src`, `CLI` = `apps/sunipdf-cli/src`

---

## 1. Objectif et comportement visible

Rendre la **structure** du document éditable et cohérente, sans toucher au rendu des pages.

| Ce que voit l'utilisateur | Détail | Correspondance |
|---|---|---|
| **Panneau Signets (F6) éditable** | Clic = sélection + navigation (comme aujourd'hui). Clic droit : *Renommer*, *Pointer vers la page courante*, *Ajouter un signet après / dans celui-ci*, *Monter*, *Descendre*, *Imbriquer*, *Remonter d'un niveau*, *Supprimer*. Double-clic ou « Renommer » : édition en place (Entrée valide, Échap annule). Bouton « + » dans l'en-tête et commande **Ctrl+B** : boîte « Nouveau signet » (titre, emplacement, vue à l'ouverture), la destination est la page courante. Un signet dont la page n'existe plus est grisé avec une infobulle, reste sélectionnable et supprimable. | PDFCraft « Edit a PDF » (structure) ; `FEATURE_MATRIX.md` : « Signets : édition planifiée » |
| **Outil « Créer un lien » (K)** | Glisser un rectangle sur la page → boîte « Créer un lien » : *page du document* (numéro + vue) ou *adresse web* (http, https, mailto, tel). Avec l'outil actif, les liens de la page sont surlignés ; clic = sélection, barre flottante *Modifier* / *Supprimer*. | PDFCraft « Edit a PDF » ; `FEATURE_MATRIX.md` : « Liens : planifié (P3) » |
| **Suivre un lien** | Avec l'outil Sélection, le curseur devient une main au-dessus d'un lien ; clic = aller à la page, ou confirmation puis ouverture dans le navigateur pour une URL. (Aujourd'hui les liens d'un PDF ne sont pas cliquables dans SuniPDF : aucune occurrence de « link » dans `apps/sunipdf-desktop/src` hors `hyperlink_to` de la boîte À propos, la couleur de lien du thème et l'étiquette « Lien » du panneau Commentaires.) | lecteur |
| **Étiquettes de pages** | Menu Pages / outil « Étiquettes de pages… » : plages (à partir de la page N, style 1/i/I/a/A/aucun, préfixe, premier numéro) avec aperçu `i, ii, iii, 1, 2…`, Appliquer / Supprimer toutes. | PDFCraft « Organize pages » ; `FEATURE_MATRIX.md` : « Étiquettes de pages : planifié (P3) » |
| **Cohérence automatique** | Supprimer des pages retire les signets et liens internes devenus orphelins (les enfants vivants remontent d'un niveau) et décale les plages d'étiquettes. Déplacer une page **conserve** signets et liens (déplacement sans perte). Extraire / diviser conserve les signets des pages retenues (renumérotés). Combiner crée un signet par fichier (titre = nom du fichier) contenant le plan du fichier. Tout est annulable d'un seul Ctrl+Z (même instantané que l'opération de pages). | PDFCraft « Organize pages », « Combine files » ; `FEATURE_MATRIX.md` : « Fusion avec signets », « Extraction préservant liens » |
| **CLI** | `bookmark-add / -rename / -remove / -move / -dest`, `links`, `link-add`, `link-remove`, `page-labels`, `set-page-labels`, `merge --no-file-bookmarks` (section 6). | automatisation (D009) |

Corrigé au passage (bug existant, prérequis de l'édition) : `Document::bookmarks()` **plafonne la profondeur à 1** (voir 3.1) ;
l'indentation des plans à 3 niveaux est donc fausse aujourd'hui.

## 2. Faisabilité

**Verdict : partiel** — le cœur est réalisable hors ligne, sans nouvelle dépendance (lopdf est déjà dans le workspace, PDFium
déjà chargé), mais des éléments sont écartés explicitement en 2.2.

### 2.1 Ce que les bibliothèques permettent réellement (vérifié dans les sources)

| Besoin | PDFium / pdfium-render 0.9.4 | Conséquence |
|---|---|---|
| Lire les signets | `PdfBookmarks::iter()` (`PR/pdf/document/bookmarks.rs:104`, parcours préfixe : le voisin suivant est empilé avant le premier enfant, `bookmarks.rs:213-238`), `PdfBookmark::title/destination/action` (`PR/pdf/document/bookmark.rs:90,149,131`) | OK pour la liste affichée |
| **Écrire** des signets | **Impossible** : les liaisons `FPDFBookmark_*` se limitent à `GetFirstChild/GetNextSibling/GetTitle/GetCount/Find/GetDest/GetAction` (`PR/bindings.rs:5846-5974`) ; aucun `set_title`, `add_bookmark`, `FPDFBookmark_Set/Add` dans tout `PR` (recherche exhaustive) | Route lopdf obligatoire |
| Créer un lien vers une **page** | **Impossible** : `create_link_annotation(uri)` (`PR/pdf/document/page/annotations.rs:250-266`) appelle `set_link` (`annotation/link.rs:76-87`) → `FPDFAnnot_SetURI` (`bindings.rs:4746`), URI seulement ; la liste complète `FPDFAnnot_*` (`bindings.rs:3568-4838`) ne contient aucun setter de `/Dest` ni d'action GoTo | Route lopdf pour les deux types (un seul chemin de code) |
| Lire un lien (cible) | `PdfPageAnnotation::as_link_annotation` (`annotation.rs:588`) → `PdfPageLinkAnnotation::link` (`annotation/link.rs:48`) → `PdfLink::action/destination` (`PR/pdf/link.rs:51,69`), `PdfAction::{Uri, LocalDestination}` (`PR/pdf/action.rs`), `PdfActionUri::uri` (`action/uri.rs:30`), `PdfActionLocalDestination::destination` (`local_destination.rs:28`), `PdfDestination::page_index/view_settings` (`PR/pdf/destination.rs:276,289`), `PdfDestinationViewSettings` (`destination.rs:23`) | Listage via PDFium (rapide, résout les destinations nommées) |
| Supprimer un lien | `delete_annotation` existant (`CORE/annot.rs:198` → `PdfPageAnnotations::delete_annotation`, `PR/.../annotations.rs:551`) | Réutilisé tel quel |
| Étiquettes de pages | Lecture seule : `PdfPage::label()` (`PR/pdf/document/page.rs:241`) alimenté par `FPDF_GetPageLabel` ; le commentaire de `PR/pdf/document/pages.rs:451` le dit : « Pdfium does not currently include an FPDF_SetPageLabel() » | Écriture par lopdf (arbre de nombres `/PageLabels`) |
| Déplacer une page sans la recréer | `FPDF_MovePages` existe dans le trait (`PR/bindings.rs:2457`) mais `PdfDocument::handle()` est `pub(crate)` (`PR/pdf/document.rs:206`) : **inaccessible** depuis SuniPDF. Le `move_page` actuel (copie vers un document temporaire, suppression, recopie ; `CORE/pages.rs:219-243`) **recrée** l'objet page | Déplacement sans perte par lopdf (3.6) |

lopdf 0.45 fournit tout le nécessaire pour l'écriture : `Document::load_mem` (`LP/reader.rs:95`), `is_encrypted` (`LP/document.rs:268`),
`catalog_mut` (`document.rs:525`), `page_iter/get_pages` (`document.rs:537/533`, itérateur `PageTreeIter` `document.rs:856`),
`get_dictionary(_mut)` (`document.rs:202/207`), `get_object(_mut)` (`document.rs:165/178`), `add_object/new_object_id/set_object/remove_object`
(`LP/creator.rs:21/15/28/36`), `Dictionary::get/get_mut/set/remove/has/get_deref` (`LP/object.rs:344-392`), `Object::string_literal`
(`object.rs:137`), `save_to` (`LP/writer.rs:22`, déjà enveloppé par `security::save`).

On **n'utilise pas** :
- `Document::add_bookmark/build_outline` (`LP/bookmarks.rs:31,124`) : reconstruit un plan de zéro (tout en `GoTo /Fit`, couleurs forcées) et ne lit pas l'existant ;
- `get_outlines` (`LP/outlines.rs:36`) : indexation `obj_array[0]` sans garde (`outlines.rs:127-128`) et `unwrap` (`LP/destinations.rs:65`) → panique possible sur PDF hostile ;
- `Document::delete_object` (`LP/processor.rs:83`) ni `prune_objects` (`processor.rs:66`) dans une boucle : ils parcourent tout le graphe (`traverse_objects`, `document.rs:224`, avec `refs.contains` linéaire) → quadratique. On utilise `doc.objects.remove(&id)` / `remove_object`.

Pièges lopdf à retenir : `From<&str>` et `From<String>` pour `Object` produisent un **`Object::Name`** (`LP/object.rs:100-110`), pas une chaîne ; texte PDF = `Object::string_literal` ou UTF-16BE avec BOM (comme `security::set_or_remove`, `CORE/security.rs:197-216`) ; `usize` n'a pas de `From<usize>` pour `Object` (caster en `i64`) ; `Real` est un `f32` (`object.rs:42`).

### 2.2 Écarté volontairement (limites à documenter dans `KNOWN_ISSUES.md`)

1. **Documents chiffrés : édition de structure refusée** (`EngineError::SecurityRestricted`, message existant `error.security`). `security::load` (`CORE/security.rs:78-94`) déchiffre puis la sauvegarde lopdf écrit un PDF **en clair** : appliquer ce schéma à un document chiffré supprimerait silencieusement la protection à l'enregistrement. (Défaut déjà présent dans `BytesOp::SetMetadata` et `merge_content_streams`, `CORE/edit.rs:402-420` ; à signaler à l'orchestrateur, hors de cette tranche.) La **lecture** (étiquettes, listage) reste permise. Le déplacement de page retombe sur l'ancien chemin PDFium ; la réparation après suppression est ignorée (journal `warn`).
2. **Destinations nommées** : lues et résolues (arbre `/Names /Dests` et dictionnaire `/Dests`), conservées telles quelles sur les éléments non modifiés ; jamais créées ni éditées.
3. **Vue de destination** : quatre modes (zoom hérité, zoom courant, page entière, largeur) ; pas de coordonnées `left/top` choisies à la souris.
4. **Style des signets** (`/C` couleur, `/F` gras/italique) : conservé s'il existe, non éditable. Pas de repli/dépli persistant dans l'UI (l'état ouvert/fermé `/Count` est conservé).
5. **Liens** : rectangle libre (pas d'ancrage au texte) ; types créés = page + URL (http, https, mailto, tel) ; les actions `Launch`, `GoToR`, JavaScript existantes sont listées « Autre », supprimables, non modifiables. Les liens sont créés **sans bordure ni apparence** (`/Border [0 0 0]`) : visibles dans SuniPDF par surimpression, invisibles dans le rendu (PDFium ne dessine pas un lien sans apparence : à vérifier, test 10). Mêmes limites de coordonnées que les autres outils sur pages pivotées / `CropBox` décalée (`annotate::to_pdf`, `DESK/annotate.rs:99`, ignore `/Rotate`).
6. **Insertion de pages d'un autre fichier** (`insert_pages_from`) : les signets du fichier source ne sont pas importés (comportement d'un « Insérer des pages »).
7. **Étiquettes** : positionnelles (3.7) ; non conservées par extraction/fusion ; non affichées dans les miniatures ni dans le champ de page (roadmap).
8. **Gros fichiers** : la réparation après suppression/insertion est ignorée au-delà de 256 Mo (`STRUCTURE_REPAIR_MAX_BYTES`), journal `warn` ; les signets morts restent grisés.
9. **Liens GoTo internes à l'import** (extraction, fusion, insertion) : conditionnel au résultat de l'étape 0 (hypothèse H5).
10. **qpdf absent de ce poste** (`which qpdf` vide ; `KNOWN_ISSUES.md` A4/A10) : la sortie est vérifiée par relecture PDFium **et** relecture lopdf (structure `/First /Last /Next /Prev /Parent /Count`) ; `qpdf --check` sur les sorties de test reste à brancher avec SUNI-025.

---

## 3. API moteur (`sunipdf-core`)

### 3.0 Principes communs

- **Un seul schéma pour toute écriture de structure**, identique à `Document::merge_content_streams` (`CORE/edit.rs:402-420`) :
  ```rust
  let bytes = self.snapshot()?;                       // PDFium save_to_bytes (CORE/pages.rs:246)
  let mut doc = structure::load_plain(&bytes)?;       // lopdf ; SecurityRestricted si chiffré
  /* mutation lopdf */
  let out = security::save(doc)?;                     // CORE/security.rs:96
  self.restore(out)?;                                 // CORE/pages.rs:266 : PdfDocument neuf + refresh_page_sizes
  ```
- **Pas de `regenerate_content`** : `restore` recharge un `PdfDocument` à partir des octets (`pages.rs:266-273`) ; les pages sont ré-analysées. La régénération n'est requise que pour des modifications d'objets de page *en place* (`edit.rs:242…`). Aucune opération de cette tranche ne modifie un objet de page PDFium en place, sauf `delete_annotation` (déjà existant, qui régénère selon la stratégie de la page, `annotations.rs:551-590`) ; le `snapshot()` qui suit l'inclut.
- **Annulation** : toutes les opérations s'exécutent dans `State::edit` (`DESK/worker.rs:372-398`) : instantané des octets avant `apply`, restauration si `apply` échoue, `push_undo` si succès. Les réparations de cohérence (3.5) vivent **dans** l'opération de pages elle-même : un seul instantané couvre l'opération PDFium et la réparation lopdf → un Ctrl+Z annule les deux. Coût mémoire : pic transitoire ≈ 3 à 4 fois la taille du document (entrée d'annulation + `bytes` + graphe lopdf + `out`) ; le plafond de 512 Mo du journal d'annulation (`worker.rs:18-19`) reste la limite.
- **Fil d'exécution** : toutes ces méthodes appartiennent à `Document` (thread moteur uniquement) ; les types de données publics (`OutlineOp`, `DestView`, `LinkInfo`, `LinkTarget`, `PageLabelRange`…) sont de simples valeurs, donc `Send` (nécessaire pour traverser `mpsc` dans `Request`/`Response`).
- **Nouveau module privé** `CORE/structure.rs` : `pub(crate) fn load_plain(bytes: &[u8]) -> Result<lopdf::Document, EngineError>` = `lopdf::Document::load_mem` (`LP/reader.rs:95`), erreur lopdf → `EngineError::InvalidFormat`, `doc.is_encrypted()` (`LP/document.rs:268`) → `EngineError::SecurityRestricted`. Les chemins en **lecture seule** (étiquettes, listage) peuvent continuer à utiliser `security::load(&bytes, self.password())` qui déchiffre en mémoire sans rien réécrire.

### 3.1 Correctif préalable : profondeur des signets (`CORE/security.rs:306-326`)

Bug vérifié : `PdfBookmark::parent()` (`PR/pdf/document/bookmark.rs:164-168`) reconstruit le parent avec `parent: None`, donc la boucle `while let Some(p) = parent { depth += 1; parent = p.parent(); }` s'arrête à **1**. Un plan à trois niveaux est affiché à deux. Correction (l'égalité et le hachage de `PdfBookmark` reposent sur le seul handle, `bookmark.rs:30-60`) :

```rust
// security.rs — remplace le corps de Document::bookmarks
impl Document<'_> {
    pub fn bookmarks(&self) -> Vec<Bookmark> { bookmarks_of(&self.inner) }
}
/// Utilisable aussi sur un document source sans l'envelopper (fusion, 3.6).
pub(crate) fn bookmarks_of(doc: &PdfDocument<'_>) -> Vec<Bookmark> {
    let mut depths: std::collections::HashMap<PdfBookmark<'_>, usize> = Default::default();
    doc.bookmarks().iter().map(|b| {
        // Parcours préfixe : le parent a déjà été vu ; `parent()` n'a pas de grand-parent mais Eq/Hash suffisent.
        let depth = b.parent().map_or(0, |p| depths.get(&p).map_or(1, |d| d + 1));
        depths.insert(b.clone(), depth);
        Bookmark {
            title: b.title().unwrap_or_default(),
            page: b.destination().and_then(|d| d.page_index().ok()).map(|i| i as usize),
            depth,
        }
    }).collect()
}
```
Test : le fixture `bookmarks-links.pdf` (section 8) a un élément de profondeur 2 ; `assert_eq!(depths, [0,1,0,1,2,1,0])`.

### 3.2 `CORE/outline.rs` (nouveau) — modèle à plat, lecture/écriture, opérations

**Types publics**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestView { Inherit, Zoom(f32), Fit, FitWidth }          // [/XYZ null null null|z], [/Fit], [/FitH null]
impl DestView { pub(crate) fn to_objects(self) -> Vec<lopdf::Object> } // Name + Object::Null / Object::Real(f32) (LP/object.rs:39,42)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind { Up, Down, Indent, Outdent }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place { End, After(usize), FirstChildOf(usize) }        // indices = ordre préfixe de Document::bookmarks()

#[derive(Debug, Clone, PartialEq)]
pub enum OutlineOp {
    Add { place: Place, title: String, page: usize, view: DestView },
    Rename { index: usize, title: String },
    Delete { index: usize },                                       // supprime l'élément ET ses descendants
    SetDestination { index: usize, page: usize, view: DestView },
    Move { index: usize, how: MoveKind },
}
```

**Fonctions publiques**

```rust
/// Octets -> octets (tests, CLI hors Document). Refuse les documents chiffrés.
pub fn edit_outline(input: &[u8], op: &OutlineOp) -> Result<Vec<u8>, EngineError>;
/// Remplace TOUT le plan par une liste à plat (extraction, fusion). Vue = Inherit. Profondeurs normalisées (<= précédente + 1).
pub fn set_outline(input: &[u8], items: &[Bookmark]) -> Result<Vec<u8>, EngineError>;
/// Pures (testables sans PDFium), utilisées par l'UI pour griser les actions et suivre la sélection.
pub fn can_move(depths: &[usize], index: usize, how: MoveKind) -> bool;
pub fn selection_after(depths: &[usize], op: &OutlineOp) -> Option<usize>;
/// Extraction : signets dont la page est retenue + leurs ancêtres (qui prennent la page du premier descendant retenu), pages renumérotées.
pub fn select_for_pages(marks: &[Bookmark], pages: &[usize]) -> Vec<Bookmark>;
/// Fusion : décale les pages de `page_offset` et les profondeurs de `extra_depth` ; supprime les éléments sans page qui n'ont aucun descendant avec page.
pub fn rebase(marks: &[Bookmark], page_offset: usize, extra_depth: usize) -> Vec<Bookmark>;

impl Document<'_> {
    /// Applique `op` sous le schéma 3.0. Vérifie d'abord que la lecture lopdf du plan est alignée sur celle de PDFium
    /// (même nombre d'éléments et mêmes profondeurs que `self.bookmarks()`), sinon `EngineError::Other("outline not editable")`.
    pub fn edit_bookmarks(&mut self, op: &OutlineOp) -> Result<(), EngineError>;
}
```

**Modèle interne** (jamais exposé)

```rust
pub(crate) struct Item {
    id: Option<lopdf::ObjectId>,        // objet existant à réutiliser (conserve /C /F /SE /A…), None = nouvel élément
    depth: usize,
    new_title: Option<String>,          // Some = écrire /Title (nouveau ou renommé) ; None = laisser l'original (encodage intact)
    open: bool,                         // signe de /Count (nouveaux éléments : true)
    state: DestState,                   // classification de la destination d'origine
    new_dest: Option<(usize, Vec<lopdf::Object>)>, // Some((page, queue de tableau)) = écrire /Dest et retirer /A
}
pub(crate) enum DestState { Page(usize), Dead, Keep }   // Dead : tableau explicite dont la page n'est plus dans l'arbre ; Keep : URI, Launch, nommée non résolue…
```

**Lecture** `read_items(doc: &lopdf::Document, pages: &[ObjectId]) -> Vec<Item>` :
1. `root = catalog["Outlines"]` (référence, via `doc.catalog()`, `LP/document.rs:516`) ; absent ou sans `/First` → liste vide.
2. Parcours **préfixe** itératif (pile `(ObjectId, depth)`, voisin `/Next` empilé avant le premier enfant `/First`, exactement comme `PR/.../bookmarks.rs:213-238`), `visited: HashSet<ObjectId>` contre les cycles, profondeur maximale 64.
3. Pour chaque nœud (`doc.get_dictionary(id)`) : `open = /Count > 0` ; destination = `/Dest` sinon `/A` avec `/S /GoTo` puis `/D` ; tableau explicite dont `[0]` est une référence : dans `pages` → `Page(index)`, sinon `Dead` ; chaîne/nom → `resolve_named(doc, name)` (dictionnaire `/Dests` du catalogue, puis arbre `/Names /Dests` avec `/Kids` et `/Names`, profondeur ≤ 32, valeur = tableau ou dictionnaire `/D`) puis même classification ; tout le reste → `Keep`.
4. Aucun décodage de titre n'est nécessaire (PDFium fournit les titres affichés) : économie du décodage PDFDocEncoding/UTF-16.

**Écriture** `write_items(doc: &mut lopdf::Document, items: &[Item], pages: &[ObjectId]) -> Result<(), EngineError>` :
1. Attribuer un `ObjectId` à chaque élément (`item.id` ou `doc.new_object_id()`, `LP/creator.rs:15`).
2. Reconstruire parent/prev/next/first/last avec une pile de profondeurs ; `/Count` = ± nombre de descendants visibles (`visible(n) = Σ enfants (1 + si ouvert visible(enfant))`, signe négatif si `!open`, clé absente pour une feuille).
3. Pour chaque élément : partir du dictionnaire existant (`doc.get_dictionary(id).cloned()`) ou d'un dictionnaire vide ; poser `Parent/Prev/Next/First/Last/Count` ; si `new_title` → `/Title` via `security::text_string` (voir 3.8) ; si `new_dest = Some((page, rest))` → `/Dest = [Reference(pages[page]), rest…]` (erreur `PageOutOfRange(page)` si hors bornes) et **retirer `/A`** ; stocker avec `doc.set_object(id, dict)` (`creator.rs:28`).
4. Supprimer les ids de l'ancien plan qui ne sont plus utilisés : `doc.objects.remove(&id)` (pas `delete_object`, voir 2.1).
5. Racine : réutiliser `/Outlines` existant ou `add_object`, poser `/Type /Outlines`, `/First`, `/Last`, `/Count` (somme des éléments racine visibles) ; liste vide → retirer `/Outlines` du catalogue (`catalog_mut`, `document.rs:525`) et supprimer la racine.

**Opérations à plat** (fonctions pures sur `&mut Vec<Item>`, `subtree_end(i)` = premier `j > i` avec `depth[j] <= depth[i]`) :
- `Add` : `Place::End` → `(at = len, depth = 0)` ; `After(i)` → `(subtree_end(i), depth[i])` ; `FirstChildOf(i)` → `(i + 1, depth[i] + 1)` ; l'élément suivant doit vérifier `depth <= new.depth + 1` (garanti par construction, vérifié par `check_structure`). Titre rogné, vide → `Other("empty title")`. Nouvel élément : `new_title = Some`, `new_dest = Some((page, view.to_objects()))`, `open = true`. Si le parent était une feuille, `open` du parent est mis à `true`.
- `Rename` : `new_title = Some`. `SetDestination` : `new_dest = Some`, `state = Page(page)`.
- `Delete(i)` : retire `i..subtree_end(i)`.
- `Move::Up/Down` : échange le bloc de `i` avec le bloc du frère précédent / suivant (frère = même profondeur sans rencontrer de profondeur inférieure) ; absent → `Other("cannot move")`.
- `Move::Indent` : exige un frère précédent (`i > 0 && depth[i-1] >= depth[i]`) ; `depth += 1` sur le bloc ; le frère précédent passe `open = true`.
- `Move::Outdent` : exige `depth[i] > 0` ; **sémantique arbre** : le bloc devient frère *suivant* de son parent, placé juste après le sous-arbre du parent (les frères suivants restent enfants du parent ; ordre à plat modifié).
- `check_structure(items)` après chaque opération : `depth[0] == 0` et `depth[k] <= depth[k-1] + 1`, sinon `Other("invalid outline structure")` (défense en profondeur ; ne doit jamais se produire).
- Erreurs d'indice : `Other(format!("bookmark index {i} out of range"))` ; page hors bornes : `PageOutOfRange(page)`.
- `selection_after` : `Add` → position d'insertion ; `Rename/SetDestination` → `index` ; `Delete` → `None` ; `Move` → nouvelle position du bloc (les fonctions de déplacement renvoient la nouvelle position).

**Extraction / fusion (fonctions pures)** :
- `select_for_pages(marks, pages)` : `map(old) = pages.iter().position(|p| *p == old)` (première occurrence si doublons) ; un élément est retenu si sa page est dans `pages`, ou si l'un de ses descendants contigus l'est (parcours depuis la fin) ; un ancêtre retenu sans page mappée prend la page du premier descendant retenu ; profondeurs inchangées (les ancêtres sont tous retenus) ; éléments sans page et sans descendant retenu : supprimés.
- `rebase(marks, offset, extra)` = `select_for_pages(marks, 0..n)` puis `page += offset`, `depth += extra`.

### 3.3 `CORE/links.rs` (nouveau)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget { Page { page: usize, view: DestView }, Uri(String), Other }
#[derive(Debug, Clone, PartialEq)]
pub struct LinkInfo { pub index: usize /* index d'annotation PDFium, base 0 */, pub rect: PointRect, pub target: LinkTarget }

impl Document<'_> {
    /// Liens de la page (PDFium). `index` est utilisable avec `delete_annotation`.
    pub fn links(&self, page_index: usize) -> Result<Vec<LinkInfo>, EngineError>;
    /// Crée un lien (lopdf). Rectangle normalisé, >= 2 pt de côté ; page cible < page_count ; URI validée par `check_uri`.
    pub fn add_link(&mut self, page_index: usize, rect: PointRect, target: &LinkTarget) -> Result<(), EngineError>;
    /// Remplace le lien d'index PDFium `index` : vérifie que c'est bien un lien (`links()`), `delete_annotation` puis `add_link`.
    pub fn replace_link(&mut self, page_index: usize, index: usize, rect: PointRect, target: &LinkTarget) -> Result<(), EngineError>;
}
pub fn check_uri(uri: &str) -> Result<String, EngineError>;  // schémas http/https/mailto/tel, <= 2048 octets, sans contrôle ; espaces et non-ASCII encodés en %XX
```

**`links()`** — appels réels : `let page = self.page_handle(page_index)?` (`CORE/pages.rs:282`), `page.annotations().iter().enumerate()` (comme `CORE/annot.rs:178-195`, même numérotation que `delete_annotation`), filtre `a.as_link_annotation()` (`PR/.../annotation.rs:588`), rectangle par `a.bounds()` (déjà utilisé `annot.rs:182`), cible :
```rust
match link_annot.link() {                                   // annotation/link.rs:48
    Ok(link) => match link.action() {                       // pdf/link.rs:51
        Some(PdfAction::Uri(u)) => u.uri().map(LinkTarget::Uri).unwrap_or(LinkTarget::Other),   // action/uri.rs:30
        Some(PdfAction::LocalDestination(l)) => dest_target(l.destination().ok()),             // local_destination.rs:28
        Some(_) => LinkTarget::Other,
        None => dest_target(link.destination()),                                               // pdf/link.rs:69 (/Dest direct)
    },
    Err(_) => LinkTarget::Other,
}
// dest_target(Option<PdfDestination>) : page_index() (destination.rs:276) -> Page { page, view: view_from_pdfium(view_settings()) } sinon Other.
// view_from_pdfium : SpecificCoordinatesAndZoom(_, _, Some(z)) -> Zoom(z) ; ( _, _, None) -> Inherit ; FitPageToWindow -> Fit ; FitPageHorizontallyToWindow(_) -> FitWidth ; autres -> Inherit  (destination.rs:23-60)
```
**`add_link`** — schéma 3.0 + lopdf :
1. `pages: Vec<ObjectId> = doc.page_iter().collect()`, `page_id = pages[page_index]` (`PageOutOfRange` sinon).
2. Dictionnaire : `/Type /Annot`, `/Subtype /Link`, `/Rect [l b r t]` (4 `Object::Real`), `/Border [0 0 0]` (aucun cadre visible dans les autres lecteurs), `/F 4` (impression), `/P` = référence de la page ; cible page → `/Dest [Reference(pages[t]), …view.to_objects()]` ; URI → `/A << /S /URI /URI (…) >>` (`Object::string_literal`, octets ASCII après `check_uri`).
3. `let id = doc.add_object(dict)` puis ajout à `/Annots` de la page : tableau direct → `push(Reference(id))` ; référence vers un tableau → `doc.get_object_mut(r)?.as_array_mut()?.push(…)` ; absent → `page_dict.set("Annots", vec![Reference(id)])`. Penser à terminer l'emprunt de `get_dictionary_mut` avant le second accès (copier la clé `/Annots` d'abord).
4. Aucune apparence (`/AP`) : PDFium ne dessine pas un lien sans `/AP` (à vérifier par le test de rendu identique, section 8, test 10).

**`replace_link`** : `links(page)` → trouver `index` (sinon `Other("not a link")`) ; `self.delete_annotation(page, index)?` (PDFium, en mémoire) ; `self.add_link(..)`. Atomique du point de vue de l'utilisateur : une seule entrée d'annulation (`State::edit`).

**`prune_dead_links(doc: &mut lopdf::Document, live: &HashSet<ObjectId>) -> usize`** (pour 3.5) : pour chaque page vivante, parcourir `/Annots` (tableau direct ou référence) ; pour chaque annotation `/Subtype /Link` dont `/Dest` (ou `/A /S /GoTo /D`) est un tableau explicite à référence morte, ou un nom/chaîne résolu par `resolve_named` vers une page morte : retirer l'entrée du tableau et `doc.objects.remove(&id)`. Les autres sous-types, les liens URI et les liens non résolus ne sont jamais touchés.

### 3.4 `CORE/labels.rs` (nouveau)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle { None, Decimal, RomanUpper, RomanLower, AlphaUpper, AlphaLower }   // /S absent, D, R, r, A, a
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLabelRange { pub start_page: usize /* base 0 */, pub style: LabelStyle, pub prefix: String, pub first_number: u32 /* >= 1 */ }

/// Pure : étiquette d'une page. Avant la première plage : numéro décimal page+1. Romain > 3999 : repli décimal. Alpha : A..Z, AA..ZZ, AAA… (répétition, comme la norme).
pub fn format_label(ranges: &[PageLabelRange], page: usize) -> String;

impl Document<'_> {
    pub fn page_labels(&self) -> Result<Vec<PageLabelRange>, EngineError>;          // lecture lopdf (security::load, déchiffre en mémoire) de /PageLabels
    pub fn set_page_labels(&mut self, ranges: &[PageLabelRange]) -> Result<(), EngineError>; // schéma 3.0 ; vide = retirer /PageLabels
    pub fn page_label(&self, page_index: usize) -> Result<Option<String>, EngineError>;      // PDFium : self.page_handle(i)?.label() (page.rs:241), pour les tests et la CLI
}
```
- **Lecture** : `catalog["PageLabels"]` (direct ou référence) ; arbre de nombres : `/Nums [k v k v…]` et `/Kids` récursifs (profondeur ≤ 32, `/Limits` ignoré) ; `v` = dictionnaire direct ou référence ; `/S` (nom), `/P` (chaîne : BOM FEFF → UTF-16BE, sinon Latin-1), `/St` (entier, défaut 1). Résultat trié par `start_page`.
- **Validation à l'écriture** : liste non vide → première plage `start_page == 0` (sinon `Other("first range must start at page 1")`), débuts strictement croissants et `< page_count` (sinon `PageOutOfRange`), `first_number >= 1`.
- **Écriture** : `catalog_mut().set("PageLabels", Dictionary{ "Nums" => [Integer(start), Dictionary{S?, P?, St?}, …] })` (dictionnaire direct, arbre plat) ; `/St` omis si 1 ; `/P` omis si vide ; style `None` → pas de `/S`.
- **Décalage positionnel** `shift_ranges(ranges, &PageChange) -> Vec<PageLabelRange>` (pure, `pub(crate)`) : `Deleted(D)` → `start' = start − |{d ∈ D : d < start}|` (si la page de départ est supprimée, c'est la page suivante qui devient début) ; plages dont les débuts coïncident : la dernière gagne ; plages `>= nouveau page_count` retirées ; `Inserted{at,count}` → `start' = start + count` si `start >= at`. Le premier début `0` est réinséré (Décimal) à l'écriture si besoin.

### 3.5 `CORE/structure.rs` — réparation après suppression / insertion

```rust
pub(crate) enum PageChange<'a> { Deleted(&'a [usize]), Inserted { at: usize, count: usize } }
pub(crate) const STRUCTURE_REPAIR_MAX_BYTES: usize = 256 * 1024 * 1024;

impl Document<'_> {
    /// Au mieux : n'échoue jamais l'opération de pages (journal `log::warn!`) ; ne fait rien si rien n'est à réparer.
    pub(crate) fn repair_structure(&mut self, change: PageChange<'_>);
}
```
Algorithme (`try_repair`) :
1. `bytes = self.snapshot()?` ; ignorer si `bytes.len() > STRUCTURE_REPAIR_MAX_BYTES`.
2. **Garde bon marché** `needs_repair(bytes: &[u8], change: &PageChange<'_>) -> bool` (fonction pure, testée sur des octets littéraux) : `Deleted` → `mentions(bytes, b"/Outlines") || mentions(bytes, b"/PageLabels") || mentions(bytes, b"/Link")` ; `Inserted` → seulement `/PageLabels` (`mentions` = recherche d'octets ; PDFium écrit les dictionnaires en clair, H6). Faux positif = un aller-retour inutile ; faux négatif impossible tant que PDFium n'écrit pas de flux d'objets (vérifié par le test 0).
3. `doc = structure::load_plain(&bytes)` ; chiffré → ignorer (journal).
4. `pages: Vec<ObjectId> = doc.page_iter().collect()` ; `live: HashSet<ObjectId>`.
5. `Deleted` : `items = read_items(..)`, retirer les éléments `Dead` en remontant d'un niveau leurs descendants contigus (`prune_dead`), `write_items` si modifié ; `prune_dead_links` ; décaler les étiquettes. `Inserted` : décaler les étiquettes.
6. Si au moins un changement : `security::save(doc)` puis `self.restore(out)`.

> Note : PDFium laisse l'objet de la page supprimée dans le fichier (orphelin, hors de l'arbre `/Kids`) : une destination qui le référence est donc « morte » mais résoluble par lopdf — la détection se fait par **appartenance à l'arbre des pages**, jamais par existence de l'objet. **[H1]** à confirmer par l'étape 0.

### 3.6 Changements dans `CORE/pages.rs`

```rust
impl Document<'_> {
    /// NOUVEAU. Valide (indices < page_count, au moins une page conservée), supprime du plus grand au plus petit via PDFium,
    /// rafraîchit les tailles, puis repair_structure(Deleted). `delete_page(i)` devient `self.delete_pages(&[i])` (signature publique inchangée).
    pub fn delete_pages(&mut self, pages: &[usize]) -> Result<(), EngineError>;
}
```
- `apply` (`DESK/worker.rs:427-440`) appelle `doc.delete_pages(&pages)` au lieu de boucler (une seule réparation par opération ; les contrôles « dernière page » restent côté moteur, le message d'`app.rs` `error.last_page` reste côté UI).

**Déplacement sans perte** (remplace le cœur de `move_page`, `pages.rs:219-243`) :
```rust
pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), EngineError> {
    /* validations inchangées */
    match self.snapshot().and_then(|b| pages::move_page_bytes(&b, from, to)).and_then(|out| self.restore(out)) {
        Ok(()) => return Ok(()),
        Err(e) => log::warn!("déplacement sans perte impossible ({e}), repli par copie"),
    }
    /* ancienne implémentation conservée telle quelle comme repli (chiffré, lopdf incapable) */
}
pub(crate) fn move_page_bytes(input: &[u8], from: usize, to: usize) -> Result<Vec<u8>, EngineError>;
```
`move_page_bytes` (lopdf, `load_plain`) :
1. `ids: Vec<ObjectId> = doc.page_iter().collect()` ; `moved = ids[from]` ; `old_parent = page["Parent"]`.
2. Retirer `Reference(moved)` du `/Kids` de `old_parent` (tableau direct ou référence) ; `/Count -= 1` sur `old_parent` et tous ses ancêtres (suivre `/Parent`, ≤ 64 niveaux).
3. `rest = ids sans moved` ; ancre : si `to < rest.len()` insérer **avant** `rest[to]` dans le `/Kids` de son parent à sa position ; sinon **après** `rest.last()`. `/Count += 1` sur le nouveau parent et ses ancêtres ; `moved./Parent = nouveau parent`.
4. Si le parent change : copier sur la page les attributs hérités absents (`Resources`, `MediaBox`, `CropBox`, `Rotate`) depuis l'ancêtre le plus proche de l'ancien parent qui les porte.
5. Toute situation inattendue (page absente de `/Kids`, `/Kids` non tableau, boucle) → `Err` → repli.
Effet : l'objet page **garde son identifiant** ; signets, liens entrants et sortants, `/StructParents`, destinations nommées restent valides. Aucune réparation nécessaire. **[H2/H3]** : l'ancien chemin les cassait probablement (à documenter par l'étape 0).

**Extraction / division** (`extract_to`, `pages.rs:135-152`) : après `save_to_bytes()` du document cible, `let marks = outline::select_for_pages(&self.bookmarks(), pages)` ; non vide → `outline::set_outline(&bytes, &marks)` ; en cas d'erreur lopdf : `log::warn!` et on écrit sans plan (l'extraction ne doit pas échouer pour un signet). `Engine::split` en hérite (il appelle `extract_to`).

**Fusion** :
```rust
#[derive(Debug, Clone, Copy)]
pub struct MergeOptions { pub file_bookmarks: bool }          // Default : true
impl Engine {
    pub fn merge(&self, inputs: &[PathBuf], output: &Path) -> Result<usize, EngineError>;               // = merge_with(.., &MergeOptions::default())
    pub fn merge_with(&self, inputs: &[PathBuf], output: &Path, opts: &MergeOptions) -> Result<usize, EngineError>;
}
```
`merge_with` : pour chaque entrée, `source = self.pdfium.load_pdf_from_file(..)` (comme `pages.rs:96`), `marks = bookmarks_of(&source)` (3.1), `n = source.pages().len()` ; si `file_bookmarks && inputs.len() > 1` : élément racine `Bookmark{ title: nom du fichier sans extension, page: Some(offset), depth: 0 }` suivi de `rebase(&marks, offset, 1)` ; sinon `rebase(&marks, offset, 0)` ; `append` puis `offset += n` ; fin : `outline::set_outline` si la liste n'est pas vide (échec → `warn`, fichier écrit sans plan).

**Insertion** (`insert_pages_from`, `pages.rs:185-216`) : après l'import et `refresh_page_sizes`, `self.repair_structure(PageChange::Inserted { at, count: selection.len() })` (étiquettes seulement).

### 3.7 Sémantique des étiquettes pendant les opérations de pages

Les plages sont **positionnelles** (clé = index de page) : suppression → 3.4 `Deleted` ; insertion → `Inserted` ; déplacement → inchangées (la page déplacée prend l'étiquette de sa nouvelle position, comme dans les lecteurs usuels) ; extraction/fusion → étiquettes non copiées (limite 2.2-7).

### 3.8 Exports, erreurs, refactor mineur

- `lib.rs` : `pub mod outline; pub mod links; pub mod labels; mod structure;` et ré-exports `pub use outline::{DestView, MoveKind, OutlineOp, Place}; pub use links::{LinkInfo, LinkTarget}; pub use labels::{LabelStyle, PageLabelRange}; pub use pages::MergeOptions;` (calqué sur `lib.rs:22-31`). `Bookmark` reste dans `security.rs`.
- `security.rs` : extraire le bloc « chaîne texte PDF » de `set_or_remove` (`security.rs:197-216`, ASCII littéral sinon UTF-16BE+BOM hexadécimal) en `pub(crate) fn text_string(v: &str) -> lopdf::Object` et le réutiliser (titres de signets, préfixes d'étiquettes). `lopdf_err`/`load`/`save` restent `pub(crate)`.
- `EngineError` : **aucune nouvelle variante** (la CLI et `error_key` restent inchangés) ; `SecurityRestricted` pour le chiffré, `PageOutOfRange` pour les pages, `Other(..)` en anglais pour le reste (indice de signet invalide, titre vide, URI refusée…).

### 3.9 Récapitulatif régénération / instantané

| Opération | Mécanisme | Régénération PDFium | Instantané d'annulation |
|---|---|---|---|
| Signets (`edit_bookmarks`), étiquettes, `add_link` | lopdf + `restore` | non (document rechargé) | `State::edit` (avant) |
| `replace_link` | `delete_annotation` PDFium + lopdf | seulement celle de `delete_annotation` | idem, une seule entrée |
| `delete_pages` | PDFium + `repair_structure` (lopdf) | non | idem, une seule entrée |
| `move_page` | lopdf + `restore` (repli : copie PDFium) | non | idem |
| `extract_to`, `merge_with` | PDFium puis `set_outline` sur les octets de sortie | non | sans objet (nouveaux fichiers, document ouvert intact) |

---

## 4. Worker (`DESK/worker.rs`)

Aucune nouvelle `BatchOp` (la fusion garde `BatchOp::Combine`, `worker.rs:561`, qui appelle `engine.merge` : le comportement « signet par fichier » vient du moteur) ; aucun nouveau `BytesOp` (les nouvelles opérations sont des méthodes de `Document`, plus simples que le schéma `BytesOp` qui re-prend un instantané, `worker.rs:526-556`).

| Élément | Où | Contenu |
|---|---|---|
| `EditOp::Outline(OutlineOp)` | enum `EditOp` (`worker.rs:76`), arme dans `apply` (`worker.rs:415`, après `DeleteAnnotation`, l. 457) | `doc.edit_bookmarks(&op)` |
| `EditOp::AddLink { page: usize, rect: PointRect, target: LinkTarget }` | idem | `doc.add_link(page, rect, &target)` |
| `EditOp::ReplaceLink { page, index, rect, target }` | idem | `doc.replace_link(page, index, rect, &target)` |
| `EditOp::SetPageLabels(Vec<PageLabelRange>)` | idem | `doc.set_page_labels(&ranges)` ; liste vide = retirer les étiquettes |
| `EditOp::Delete { pages }` (existant) | `worker.rs:427-440` | remplacer la boucle `delete_page` par `doc.delete_pages(&pages)` (le tri/dédoublonnage et le contrôle « au moins une page » passent dans le moteur) |
| `Request::Links { page }` / `Response::Links { page, list: Vec<LinkInfo> }` | enums `Request` (l. 178) / `Response` (l. 238) ; traitement dans `run` à côté de `Request::Annotations` (l. 795) | `doc.links(page).unwrap_or_default()` (comme `annotations`, `worker.rs:795-799`) |
| `Request::PageLabels` / `Response::PageLabels(Vec<PageLabelRange>)` | idem, à côté de `Request::Bookmarks` (l. 791) | `doc.page_labels()` ; erreur → `log::warn!` + `continue` (même motif que les autres requêtes sans réponse) |
| `use sunipdf_core::{…}` | `worker.rs:9-13` | ajouter `LinkInfo, LinkTarget, OutlineOp, PageLabelRange` |

Notes :
- `State::edit` (`worker.rs:372`) suffit tel quel : instantané avant, restauration en cas d'erreur, `push_undo`, `invalidate()` du cache de rendu, `DocumentChanged`. Les opérations de pages qui contiennent une réparation (3.5) restent atomiques.
- Les réponses `Links`/`PageLabels` sont des lectures : pas d'instantané, pas de `DocumentChanged`.
- `Suspend`/`Resume` (`worker.rs:731-775`) ne transportent que les octets et l'historique : rien à ajouter (le plan de signets est dans les octets).

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Commandes (registre `CORE/command.rs`)

Trois variantes ajoutées à `enum Command` (`command.rs:5-65`) et trois entrées `cmd!` (macro `command.rs:95-107`) ; `execute` et `command_icon` sont exhaustifs, le compilateur guide.

| Variante | id | clé libellé | raccourci | menu | ToolTab | couleur (`command_color`) | icône |
|---|---|---|---|---|---|---|---|
| `AddBookmark` | `bookmark.add` | `menu.edit.add_bookmark` | `Ctrl+B` (`shortcut.add_bookmark`) | `Menu::Edit` | `All`, `Read`, `EditTab` | `CAT_EDIT` | `glyph::ADD` (`\u{E710}`), repli « + » |
| `ToolLink` | `tool.link` | `menu.edit.tool_link` | `K` (`shortcut.tool_link`) | `Menu::Edit` | `All`, `EditTab` | `CAT_EDIT` | `glyph::LINK` (`\u{E71B}`), repli « 🔗 » |
| `PageLabels` | `pages.labels` | `menu.pages.labels` | aucun | `Menu::Pages` | `All`, `Pages` | `CAT_PAGES` (branche `Menu::Pages` de `shell.rs:150-188`) | `glyph::NUMBERS` (déjà défini), repli « # » |

- `icons.rs` (`glyph`, `icons.rs:11-67`) : ajouter `pub const ADD: &str = "\u{E710}"; pub const LINK: &str = "\u{E71B}";`.
- Les codepoints Segoe `E710` (Add) et `E71B` (Link) sont les valeurs MDL2/Fluent usuelles mais **non vérifiées dans ce poste** : contrôler à l'écran (capture) ; sinon le repli texte s'affiche (comportement A1).
- Insérer `ToolLink`/`AddBookmark` dans `ToolTab::commands` (`shell.rs:44-151`) : `All` après `ToggleBookmarks`/`ToolAddImage`, `Read` après `ToggleBookmarks`, `EditTab` après `ToolLine`, `Pages` après `PageNumbers`.
- `from_id` rend les trois ids disponibles pour `SUNIPDF_STARTUP_COMMANDS` / `dev-screenshot.ps1 -Commands` et la palette Ctrl+K (lecture automatique du registre).
- Raccourcis dans `handle_input` (`app.rs:~735-770`) : `key(Modifiers::COMMAND, Key::B, Command::AddBookmark); key(Modifiers::NONE, Key::K, Command::ToolLink);` (`B` et `K` non utilisés ; `Ctrl+K` = palette inchangé).
- Garde des modales (`app.rs:591-598`, la liste `if self.password_prompt.is_some() || self.show_about || …`) : ajouter `|| self.docinfo.bookmark_add.is_some() || self.docinfo.labels_open || self.annotate.links.form.is_some() || self.annotate.links.confirm_url.is_some()` pour que les touches du viewer ne se déclenchent pas sous une modale.
- `execute` (`app.rs:793+`) : `Command::AddBookmark => self.open_bookmark_dialog(None)` ; `Command::PageLabels => { labels_open = true; labels_loaded = false; worker.send(Request::PageLabels) }` ; `Command::ToolLink` rejoint le bras des outils (`app.rs:923-933`) avec `annotate.links.selected = None` et `status_msg = Some(tr(lang, "links.hint"))` (indication « Glissez sur la page… »). `Command::DeleteObject` : si `annotate.tool == Tool::Link` → `delete_selected_link()` sinon comportement actuel.

### 5.2 Panneau des signets (`DESK/docinfo.rs`)

**État** (`DocInfoState`, `docinfo.rs:9-18`, ajouter) :
```rust
pub bookmark_selected: Option<usize>,            // index préfixe dans `bookmarks`
pub bookmark_select_next: Option<usize>,         // sélection à appliquer à la prochaine liste reçue
pub bookmark_rename: Option<(usize, String)>,    // édition en place
pub bookmark_add: Option<BookmarkForm>,
pub bookmarks_busy: bool,                        // une opération est en vol : actions ignorées jusqu'à Response::Bookmarks
pub labels_open: bool, pub labels_loaded: bool,
pub labels: Vec<PageLabelRange>, pub labels_error: Option<&'static str>,

pub struct BookmarkForm { pub title: String, pub anchor: Option<usize>, pub place: PlaceChoice, pub view: ViewChoice,
                          pub page: usize, pub focus: bool, pub error: Option<&'static str> }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum PlaceChoice { End, After, Child }
#[derive(Clone, Copy, PartialEq, Eq, Default)] pub enum ViewChoice { #[default] Inherit, CurrentZoom, Fit, FitWidth }
impl ViewChoice { pub fn to_view(self, zoom: f32) -> DestView /* CurrentZoom -> Zoom(zoom) ; effective_zoom == zoom PDF (96 dpi = 100 %) */ }
```
`DocInfoState` dérive déjà `Default` (les `Option`/`Vec`/`bool` conviennent). Il est échangé par onglet (`tabs.rs:53-100`) : sélection, renommage et dialogues suivent l'onglet sans code supplémentaire ; `close_document` (`app.rs:239`) le remet à zéro sauf `bookmarks_open`.

**`bookmarks_panel`** (réécriture de `docinfo.rs:30-92`) :
- En-tête : titre, bouton fermer (existant), bouton « + » (`glyph::ADD`, infobulle `bookmarks.add`) → `execute(AddBookmark)`.
- Liste vide : `bookmarks.empty` + ligne grisée `bookmarks.empty_hint`.
- Une rangée par signet (`enumerate`), indentation `8 + depth*14`. **Toutes les rangées sont activées** (aujourd'hui `add_enabled(b.page.is_some(), …)`, `docinfo.rs:67` : un signet mort ne peut ni être sélectionné ni supprimé). Rangée `page == None` : texte grisé, infobulle `bookmarks.no_page`.
- `Button::new(text).frame(selected).selected(selected)` ; `resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr_args("a11y.bookmark_row", …)))` (`EG/response.rs:869`, `EG/data/output.rs:658`).
- `resp.clicked()` → `BmAction::Select(i)` (sélection + `go_to_page` si page) ; `resp.double_clicked()` (`EG/response.rs:237`) → `StartRename(i)` ; `resp.context_menu(|ui| …)` (`EG/response.rs:1028`, fermeture par `ui.close()`, `EG/ui.rs:1040`) avec : *Renommer*, *Pointer vers la page courante*, séparateur, *Ajouter un signet après celui-ci*, *Ajouter un sous-signet*, séparateur, *Monter*, *Descendre*, *Imbriquer*, *Remonter d'un niveau* (chacun `ui.add_enabled(can_move(&depths, i, how), …)`), séparateur, *Supprimer* (`dialog`-free : l'annulation Ctrl+Z suffit, pas de confirmation).
- Renommage : `TextEdit::singleline(&mut buf)`, focus demandé à la première image (même motif que `note_dialog`, `annotate.rs:261-263`) ; Échap → annule ; `resp.lost_focus()` (`EG/response.rs:372`) → valide si le texte rogné est non vide et différent.
- Pied du panneau : `bookmarks.hint` grisé (découvrabilité du clic droit).
- Les emprunts (`self.docinfo.bookmarks` en lecture + `bookmark_rename` en écriture) se règlent en déstructurant `let DocInfoState { bookmarks, bookmark_rename, .. } = &mut self.docinfo;` ; les actions sont collectées dans un `Option<BmAction>` puis appliquées après la boucle (motif `go` actuel, `docinfo.rs:55-90`).

**Envoi** : 
```rust
fn bookmark_send(&mut self, op: OutlineOp) {
    if self.docinfo.bookmarks_busy { return; }
    let depths: Vec<usize> = self.docinfo.bookmarks.iter().map(|b| b.depth).collect();
    self.docinfo.bookmark_select_next = outline::selection_after(&depths, &op);   // Rename/SetDestination gardent la sélection
    self.docinfo.bookmarks_busy = true;
    self.worker.send(Request::Edit(EditOp::Outline(op)));
}
```
`Response::Bookmarks(list)` (`app.rs:486-488`) devient : `docinfo.bookmarks = list; docinfo.bookmarks_busy = false; docinfo.bookmark_selected = docinfo.bookmark_select_next.take().filter(|i| *i < list.len());` (toute autre modification du document — suppression de pages, annulation — efface donc la sélection) ; `Response::OperationFailed` (`app.rs:489`) remet `bookmarks_busy = false`. Le drapeau empêche qu'un second clic utilise des indices périmés avant l'arrivée de la liste rafraîchie (déjà demandée par `DocumentChanged`, `app.rs:~450`).

**Boîte « Nouveau signet »** (`bookmark_add_dialog`, `dialog::show`, id `"bookmark-add"`, `Spec::new(.., tr("bookmarks.dialog.title"), dark, has_icons).subtitle(tr_args("bookmarks.dialog.subtitle", &[("page", n)])).icon(glyph::BOOKMARK, "🔖").width(460.0)`, conformément à D020) :
- Corps : `dialog::field_label` + champ titre (focus initial, Entrée valide) ; `ui.radio_value(&mut form.place, …)` (`EG/ui.rs:1911`) pour *À la fin / Après le signet sélectionné / Dans le signet sélectionné* (les deux derniers désactivés si `anchor == None`) ; `ComboBox` de vue (`dest.view.*`, motif `position_combo`, `tools.rs:56-65`) ; erreur éventuelle en `theme::ERROR`.
- Pied : `dialog::primary(tr("bookmarks.dialog.ok"))`, `dialog::secondary(tr("dialog.password.cancel"))` (clé existante, comme `note_dialog`), Échap.
- Soumission : titre rogné vide → `error = Some("bookmarks.dialog.error_empty")` ; sinon `bookmark_send(OutlineOp::Add { place, title, page: form.page, view: form.view.to_view(self.effective_zoom) })`, fermeture, `docinfo.bookmarks_open = true`.
- `open_bookmark_dialog(place: Option<PlaceChoice>)` mémorise `anchor = bookmark_selected`, `page = document.page` au moment de l'ouverture (la modale empêche toute navigation), place par défaut `After` si ancre sinon `End`.

### 5.3 Outil lien, surimpression, suivi de lien (`DESK/links.rs`, nouveau ; `mod links;` dans `main.rs:3-14`)

```rust
#[derive(Default)]
pub struct LinksState { pub page: Option<usize>, pub list: Vec<LinkInfo>, pub selected: Option<usize> /* LinkInfo.index */,
                        pub form: Option<LinkForm>, pub confirm_url: Option<String> }
pub struct LinkForm { pub page: usize, pub rect: PointRect, pub replace: Option<usize>, pub kind: LinkKind,
                      pub target_page: usize /* base 0 */, pub view: ViewChoice, pub url: String, pub error: Option<&'static str> }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum LinkKind { Page, Url }
```
Champ ajouté : `AnnotateState.links: LinksState` (`annotate.rs:87-94`, déjà échangé par onglet via `TabState.annotate`, `tabs.rs:19-30`).

| Fonction (`impl SuniApp`) | Rôle |
|---|---|
| `refresh_links_if_needed()` | appelée dans `document_area` juste après `refresh_annotations_if_needed()` (`app.rs:1095`) : si `links.page != Some(doc.page)` → `links.page = Some(..)`, `list.clear()`, `Request::Links { page }` (motif `annotate.rs:291-300`) |
| `on_links(page, list)` | `Response::Links` : ignorer si `page != links.page` |
| `paint_links(&painter, page_rect, page_index, page_h_pt)` | appelée après `paint_object_selection` (`app.rs:1144`) ; ne dessine que si `tool == Tool::Link` : rectangle `accent` à 1,5 px + remplissage `alpha 40` ; sélectionné : 3 px. Conversion points→écran identique à `paint_selection` (`annotate.rs:164-170`) |
| `link_hit(page, pt) -> Option<&LinkInfo>` | test de rectangle (tolérance 2 pt, comme `editor::contains`, `editor.rs:137`) |
| `link_click(ctx, page, pt)` | bras `DragEvent::Click` (`app.rs:1207-1222`) pour `Tool::Select` **et** `Tool::Link` : *Select* → `Page{page}` : `go_to_page(page)` ; `Uri(u)` : `links.confirm_url = Some(u)` ; *Link* → `links.selected = hit.index` |
| `link_finish_drag(drag)` | bras `Tool::Link` ajouté **avant** `_` dans `finish_drag` (`annotate.rs:190-225`) : si pas `tiny` → `links.form = Some(LinkForm::new(page, rect))` (cible par défaut = page suivante si elle existe) |
| `link_bar(ctx, origin)` | `Area` flottante (motif `object_bar`, `editor.rs:609-710`) quand un lien est sélectionné : libellé (`links.target.*`), boutons *Modifier* (ouvre `LinkForm` prérempli avec `replace = Some(index)`) / *Supprimer* |
| `delete_selected_link()` | `Request::Edit(EditOp::DeleteAnnotation { page, index })` (existant) |
| `link_dialog(ctx)` | `dialog::show`, id `"link"`, icône `glyph::LINK`, largeur 480 : `radio_value` Page/URL ; page = `DragValue::new(&mut v).range(1..=n)` (`EG/widgets/drag_value.rs:117`) + combo de vue ; URL = `TextEdit::singleline` avec indication ; soumission : URL validée côté UI par `sunipdf_core::links::check_uri` (erreur `links.dialog.error_url`) puis `EditOp::AddLink` (création) ou `EditOp::ReplaceLink` (édition) |
| `link_confirm_dialog(ctx)` | `dialog::show`, id `"link-confirm"`, `.warning()` : affiche l'URL, bouton *Ouvrir* → `ctx.open_url(egui::OpenUrl::new_tab(url))` (`EG/context.rs:1677`, déjà utilisé par `hyperlink_to` ; feature eframe `links` active, `Cargo.toml` racine) |

- **Invalidation immédiate** : toute émission d'`AddLink/ReplaceLink/DeleteAnnotation` fait `links.page = None; links.list.clear(); links.selected = None` pour qu'aucun clic n'utilise un `index` d'annotation périmé (l'index PDFium change dès qu'une annotation est supprimée).
- `Tool::Link` : variante ajoutée à `Tool` (`annotate.rs:10-24`), `Tool::ALL: [Tool; 12]`, `command()` → `Command::ToolLink`. **Ne pas** l'ajouter à `is_edit()` (`editor.rs:154-165`) : ainsi `paint_selection` dessine déjà le rectangle de glissement (`annotate.rs:156-188`) et `object_bar` reste masquée. `page_cursor` (`editor.rs:186-215`) : `Tool::Link` rejoint le groupe `Crosshair` ; `Tool::Select` avec `link_hit` → `CursorIcon::PointingHand`.
- La palette flottante (`annotate.rs:108-154`) boucle sur `Tool::ALL` : le bouton Lien apparaît seul (icône `command_icon(ToolLink)`).
- Pas de `DragMode`/poignées : l'édition d'un lien = *Modifier* (destination) ou le supprimer et en redessiner un ; déplacer/redimensionner la zone est hors tranche.

### 5.4 Boîte « Étiquettes de pages » (`docinfo.rs`, à côté de `protect_dialog`)

`page_labels_dialog(ctx)` : `dialog::show`, id `"labels"`, `.icon(glyph::NUMBERS, "#").width(620.0)`, sous-titre `labels.subtitle`. Corps : un `egui::Grid` d'une ligne par plage : *À partir de la page* (`DragValue 1..=page_count`, **verrouillé à 1 pour la première plage**), *Style* (`ComboBox` : `labels.style.*`), *Préfixe* (`TextEdit`), *Premier numéro* (`DragValue >= 1`), bouton supprimer (désactivé pour la première). Sous la grille : *Ajouter une plage* (démarre à la page courante, style Décimal) et l'aperçu `labels.preview` = `format_label` (`sunipdf_core::labels`) des 5 premières pages de chaque plage séparées par « , ». Pied : *Appliquer* (primaire), *Supprimer toutes les étiquettes* (secondaire → `SetPageLabels(vec![])`), *Annuler*. Validation UI : débuts strictement croissants, sinon `labels_error = Some("labels.error_order")` ; le moteur revalide. Chargement : `Response::PageLabels(list)` → `docinfo.labels = list` (vide → une plage par défaut `{0, Decimal, "", 1}`), `labels_loaded = true` ; spinner tant que `!labels_loaded`. Documents chiffrés : lecture permise, `Appliquer` renverra `SecurityRestricted` → bandeau `error.security`.

### 5.5 Tabs, annulation, états

- Tout l'état nouveau est dans `DocInfoState` et `AnnotateState`, déjà échangés par `take_tab/put_tab` (`tabs.rs:50-100`) ; la liste de signets est redemandée à la reprise (`on_resumed`, `tabs.rs:~195`) ; à la reprise d'un onglet, `links.page`/`links.list` restaurés correspondent aux octets restaurés : aucun code supplémentaire dans `put_tab` ; seul `Response::DocumentChanged` invalide la liste.
- `Response::DocumentChanged` (`app.rs:422-458`) : ajouter `self.annotate.links.page = None; self.annotate.links.selected = None;` (la liste de signets est déjà redemandée, `app.rs:~450`).
- Annulation/rétablissement : `Request::Undo/Redo` restaurent l'octet-instantané (`worker.rs:395-414`) → signets, liens, étiquettes et réparations reviennent ensemble ; `DocumentChanged` rafraîchit tout.
- Test clavier : sous une modale, les raccourcis `Ctrl+B`, `K`, `F6`… sont ignorés (garde 5.1).
- Accessibilité : clés `a11y.bookmark_row` (`widget_info` des rangées) et `a11y.link_area` (texte alternatif de la barre de lien).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`, anglais + JSON, D009)

Conventions existantes : pages en base 1, `--out` obligatoire pour toute écriture (`write_atomic` via `Document::save_as`), `--password` pour ouvrir, erreurs `{"error": code, "message": …}` sur stderr, codes de sortie 1/3/4/5 (`main.rs:651-672`). Les nouvelles sous-commandes suivent le motif de `Rotate`/`Insert`/`Move` : `Engine::new()` → `engine.open(&file, password)` → méthode de `Document` → `doc.save_as(&out)`. **Les indices de signets et de liens sont en base 1** dans la CLI (base 0 dans le moteur). Un document chiffré ouvert avec son mot de passe renvoie `{"error":"security_restricted"}` (exit 1) pour toute écriture de structure (2.2-1).

| Sous-commande | Arguments | Sortie JSON (stdout) |
|---|---|---|
| `bookmarks FILE [--password]` *(existant)* | — | `[ {"index":1,"title":"Intro","page":1,"depth":0}, … ]` — champ **`index`** ajouté (additif, compatible) ; `page` = `null` si pas de page |
| `bookmark-add FILE --title T --page N [--view V] [--after I \| --child-of I] --out OUT` | défaut : à la fin ; `V` = `inherit` (défaut), `fit`, `fit-width`, `zoom:1.5` | `{"out":…, "index": <1-based du nouveau signet>, "bookmark_count": n}` |
| `bookmark-rename FILE --index I --title T --out OUT` | — | `{"out":…, "index":I, "title":T}` |
| `bookmark-remove FILE --index I --out OUT` | supprime aussi les descendants | `{"out":…, "removed": k, "bookmark_count": n}` |
| `bookmark-move FILE --index I --to up\|down\|indent\|outdent --out OUT` | — | `{"out":…, "index": <nouvelle position>}` (`selection_after`) |
| `bookmark-dest FILE --index I --page N [--view V] --out OUT` | — | `{"out":…, "index":I, "page":N}` |
| `links FILE [--page N] [--password]` | toutes les pages sinon | `{"pages":[{"page":1,"links":[{"index":1,"rect":[l,b,r,t],"type":"page","page":3},{"index":2,"rect":[…],"type":"uri","uri":"https://…"},{"index":3,"rect":[…],"type":"other"}]}]}` (pages sans lien omises) |
| `link-add FILE --page N --rect l,b,r,t (--url U \| --to-page M [--view V]) --out OUT` | rect en points PDF, origine bas-gauche | `{"out":…, "page":N, "link_count": k}` |
| `link-remove FILE --page N --index I --out OUT` | refuse (`other`) si l'annotation n'est pas un lien | `{"out":…, "page":N, "removed": I}` |
| `page-labels FILE [--password]` | lecture | `{"ranges":[{"start_page":1,"style":"roman-lower","prefix":"","first_number":1}], "labels":["i","ii","A-1",…]}` (`labels` = `format_label`) |
| `set-page-labels FILE (--range SPEC… \| --clear) --out OUT` | `SPEC = START[:STYLE[:PREFIX[:FIRST]]]`, `STYLE` ∈ `none, decimal, roman-upper, roman-lower, alpha-upper, alpha-lower` (défaut `decimal`) | `{"out":…, "ranges": n}` |
| `merge INPUTS… --out OUT [--no-file-bookmarks]` *(option ajoutée)* | par défaut : un signet par fichier si plusieurs entrées + plans des sources décalés | `{"out":…, "page_count": n}` inchangé |

Fonctions d'analyse (tests unitaires dans le module `tests` existant, `main.rs:674`) : `parse_view(&str) -> Result<DestView, EngineError>`, `parse_rect(&str) -> Result<PointRect, EngineError>`, `parse_label_range(&str) -> Result<PageLabelRange, EngineError>` (indices/pages de départ convertis en base 0, `0` rejeté comme `parse_pages`, `main.rs:625-650`). La commande `commands` (`main.rs:267-277`) liste automatiquement `bookmark.add`, `tool.link`, `pages.labels` (63 → 66 identifiants de registre).

## 7. i18n — clés à ajouter dans `assets/i18n/fr-FR.json` **et** `en-US.json`

74 clés (380 → 454 par catalogue ; `grep -c '^  "'` donne 380 aujourd'hui dans chacun des deux fichiers). Réutilisées sans changement : `bookmarks.title|empty|page`, `tools.close`, `about.close`, `dialog.password.cancel`, `error.security`, `error.page_range`. Placeholders identiques FR/EN (test `placeholders_match_between_languages`). Propriété du catalogue : agent ui-ux (D013) ; préfixes proposés ci-dessous (`docs/I18N.md` à mettre à jour).

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.edit.add_bookmark` | Ajouter un signet… | Add Bookmark… |
| `shortcut.add_bookmark` | Ctrl+B | Ctrl+B |
| `menu.edit.tool_link` | Créer un lien | Create Link |
| `shortcut.tool_link` | K | K |
| `menu.pages.labels` | Étiquettes de pages… | Page Labels… |
| `bookmarks.add` | Ajouter un signet à la page courante | Add a bookmark to the current page |
| `bookmarks.empty_hint` | Ajoutez-en un avec le bouton + ou Ctrl+B. | Add one with the + button or Ctrl+B. |
| `bookmarks.hint` | Clic droit sur un signet pour le modifier. | Right-click a bookmark to edit it. |
| `bookmarks.no_page` | Destination introuvable (page supprimée ?) | Destination not found (page deleted?) |
| `bookmarks.rename` | Renommer | Rename |
| `bookmarks.set_dest` | Pointer vers la page courante | Point to the current page |
| `bookmarks.add_after` | Ajouter un signet après celui-ci | Add a bookmark after this one |
| `bookmarks.add_child` | Ajouter un sous-signet | Add a child bookmark |
| `bookmarks.move_up` | Monter | Move up |
| `bookmarks.move_down` | Descendre | Move down |
| `bookmarks.indent` | Imbriquer sous le signet précédent | Nest under the previous bookmark |
| `bookmarks.outdent` | Remonter d'un niveau | Move up one level |
| `bookmarks.delete` | Supprimer (avec ses sous-signets) | Delete (with its children) |
| `bookmarks.dialog.title` | Nouveau signet | New bookmark |
| `bookmarks.dialog.subtitle` | Le signet pointera vers la page {page}. | The bookmark will point to page {page}. |
| `bookmarks.dialog.name` | Titre du signet | Bookmark title |
| `bookmarks.dialog.placeholder` | Ex. : Chapitre 1 | e.g. Chapter 1 |
| `bookmarks.dialog.place` | Emplacement | Position |
| `bookmarks.dialog.place_end` | À la fin de la liste | At the end of the list |
| `bookmarks.dialog.place_after` | Après le signet sélectionné | After the selected bookmark |
| `bookmarks.dialog.place_child` | Dans le signet sélectionné (sous-signet) | Inside the selected bookmark (child) |
| `bookmarks.dialog.view` | Affichage à l'ouverture | View when opened |
| `bookmarks.dialog.ok` | Ajouter | Add |
| `bookmarks.dialog.error_empty` | Saisissez un titre. | Enter a title. |
| `dest.view.inherit` | Conserver le zoom du lecteur | Keep the reader's zoom |
| `dest.view.zoom` | Zoom actuel de SuniPDF | SuniPDF's current zoom |
| `dest.view.fit` | Page entière | Whole page |
| `dest.view.fit_width` | Largeur de la page | Page width |
| `links.dialog.title` | Créer un lien | Create link |
| `links.dialog.title_edit` | Modifier le lien | Edit link |
| `links.dialog.subtitle` | Zone cliquable sur la page {page}. | Clickable area on page {page}. |
| `links.dialog.kind_page` | Aller à une page du document | Go to a page in this document |
| `links.dialog.kind_url` | Ouvrir une adresse web | Open a web address |
| `links.dialog.target_page` | Page de destination | Destination page |
| `links.dialog.view` | Affichage à l'arrivée | View on arrival |
| `links.dialog.url` | Adresse (http, https, mailto, tel) | Address (http, https, mailto, tel) |
| `links.dialog.url_placeholder` | https://exemple.org | https://example.org |
| `links.dialog.ok` | Créer le lien | Create link |
| `links.dialog.ok_edit` | Enregistrer | Save |
| `links.dialog.error_url` | Adresse invalide : utilisez http://, https://, mailto: ou tel:. | Invalid address: use http://, https://, mailto: or tel:. |
| `links.bar.edit` | Modifier le lien | Edit link |
| `links.bar.delete` | Supprimer le lien | Delete link |
| `links.target.page` | Lien vers la page {page} | Link to page {page} |
| `links.target.url` | Lien vers {url} | Link to {url} |
| `links.target.other` | Lien (action non modifiable) | Link (action cannot be edited) |
| `links.hint` | Glissez sur la page pour tracer la zone du lien. | Drag on the page to draw the link area. |
| `links.confirm.title` | Ouvrir ce lien ? | Open this link? |
| `links.confirm.text` | Le document demande d'ouvrir cette adresse dans votre navigateur : | The document asks to open this address in your browser: |
| `links.confirm.open` | Ouvrir | Open |
| `labels.title` | Étiquettes de pages | Page labels |
| `labels.subtitle` | Numérotation affichée par les lecteurs PDF (i, ii, iii, 1, 2…). | Numbering shown by PDF readers (i, ii, iii, 1, 2…). |
| `labels.start` | À partir de la page | From page |
| `labels.style` | Style | Style |
| `labels.prefix` | Préfixe | Prefix |
| `labels.first` | Premier numéro | First number |
| `labels.add_range` | Ajouter une plage | Add range |
| `labels.remove_range` | Supprimer la plage | Remove range |
| `labels.clear` | Supprimer toutes les étiquettes | Remove all labels |
| `labels.preview` | Aperçu : {sample} | Preview: {sample} |
| `labels.apply` | Appliquer | Apply |
| `labels.error_order` | Les plages doivent commencer à des pages croissantes. | Ranges must start at increasing pages. |
| `labels.style.none` | Aucun (préfixe seul) | None (prefix only) |
| `labels.style.decimal` | 1, 2, 3… | 1, 2, 3… |
| `labels.style.roman_lower` | i, ii, iii… | i, ii, iii… |
| `labels.style.roman_upper` | I, II, III… | I, II, III… |
| `labels.style.alpha_lower` | a, b, c… | a, b, c… |
| `labels.style.alpha_upper` | A, B, C… | A, B, C… |
| `a11y.bookmark_row` | Signet : {title}, page {page} | Bookmark: {title}, page {page} |
| `a11y.link_area` | Lien vers {target} | Link to {target} |

Garde-fous : (a) `registry_keys_exist_in_both_catalogs` (`app.rs:1984`) couvre les 5 clés de registre ; (b) `ui_keys_used_by_app_exist` (`app.rs:1964`) ne lit que `app.rs` et `main.rs` (`include_str!`) : **l'étendre** à `docinfo.rs` et `links.rs` (`concat!(include_str!("app.rs"), include_str!("main.rs"), include_str!("docinfo.rs"), include_str!("links.rs"))`) après avoir vérifié qu'aucun littéral en minuscules/points/underscores n'y est autre chose qu'une clé ; (c) `scripts\check-i18n.ps1` (mêmes ensembles de clés).

## 8. Tests

### 8.1 Fixtures (`scripts/make_fixtures.py`, régénérer par `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`)

Ajouter `bookmarks_links(path)` et `nested_pages(path)` (fonctions appelées depuis `__main__`, `make_fixtures.py:84`) qui réutilisent `build()` et `stream()`. `two-pages.pdf` et `encrypted-rc4.pdf` ne changent pas.

**`bookmarks-links.pdf`** (5 pages 612×792, objets numérotés à la main comme `two_pages`) :
- Contenu de la page *k* : `r g b rg 72 500 300 150 re f` + `BT /F1 24 Tf 72 700 Td (Page k) Tj ET` ; couleurs p0 rouge `1 0 0`, p1 vert `0 0.6 0`, p2 bleu `0 0 1`, p3 jaune `1 0.9 0`, p4 magenta `1 0 1`. Pixel de contrôle à l'échelle 1,0 : `(222, 217)` (milieu du rectangle, `y = 792 − 575`).
- Objets : 1 Catalog (`/Pages 2 0 R /Outlines 20 0 R /PageMode /UseOutlines /Names << /Dests << /Names [(chap1) [7 0 R /Fit]] >> >> /PageLabels << /Nums [0 << /S /r >> 2 << /S /D /P (A-) >>] >>`) ; 2 Pages `/Kids [3 5 7 9 11] /Count 5` ; pages 3, 5, 7, 9, 11 (contenus 4, 6, 8, 10, 12) ; 13 police Helvetica ; annotations : 14 (page 0, lien `/Rect [72 600 200 620] /Border [0 0 0] /Dest [7 0 R /Fit]`), 15 (page 0, URI `https://example.org/`, `/Rect [72 560 200 580]`), 16 (page 1, `/Dest (chap1)` nommée, `/Rect [72 600 200 620]`), 17 (page 3, `/A << /S /GoTo /D [3 0 R /Fit] >>`, `/Rect [300 100 400 130]`) ; `/Annots` des pages en conséquence.
- Plan (racine 20 : `/First 21 /Last 27 /Count 7`) :
  `21 Intro` (page 0, `/Dest [3 0 R /Fit]`, enfant 22) · `22 Contexte` (`/A << /S /GoTo /D [5 0 R /XYZ 72 700 0] >>`) · `23 Chapitre 1` (`/Dest (chap1)`, enfants 24 et 26, `/Count 3`) · `24 Section 1.1` (page 3, enfant 25, `/Count 1`) · `25 Détail 1.1.a` (`[9 0 R /XYZ 72 400 1.5]`) · `26 Section 1.2` (page 4) · `27 Annexe` (`/A << /S /URI /URI (https://example.org/annexe) >>`, sans page).
  Attendu par PDFium : profondeurs `[0,1,0,1,2,1,0]`, pages `[0,1,2,3,3,4,None]` (hypothèse H7 pour les éléments 22 et 23).
- Étiquettes attendues : `i, ii, A-1, A-2, A-3`.

**`nested-pages.pdf`** (arbre de pages à deux niveaux, attributs hérités) : racine `/Kids [3 4] /Count 4` ; nœud 3 `/Parent 2 /Kids [5 6] /Count 2 /MediaBox [0 0 612 792]` ; nœud 4 `/Parent 2 /Kids [7 8] /Count 2 /MediaBox [0 0 400 400]` ; pages 5-8 sans `/MediaBox` (héritée) avec rectangle `r g b rg 20 20 100 100 re f` rouge/vert/bleu/jaune et texte `(Nested A)`…`(Nested D)`.

### 8.2 Tests unitaires (sans PDFium, `cargo test -p sunipdf-core`)

- `outline.rs` : `add_end/after/child` (positions et profondeurs), `delete_removes_subtree`, `move_up_down_swap_sibling_blocks`, `indent_requires_previous_sibling`, `outdent_moves_after_parent_subtree` (les frères suivants restent enfants du parent), `check_structure_rejects_depth_jump`, `can_move_matches_apply`, `selection_after_*`, `count_sign_and_visible_counts` (valeurs `/Count` ± attendues), `select_for_pages` (doublons, parent sans page mappée, éléments sans page), `rebase`, `read_write_round_trip_on_programmatic_doc` (document construit avec `lopdf::Document::with_version`, `add_object`, `save_to` : lire → écrire → relire = identique ; liens `/First /Last /Next /Prev /Parent` cohérents ; `/Count` ; objets orphelins supprimés ; cycle `/Next` → terminaison), `resolve_named_tree_and_dests_dict`.
- `labels.rs` : `format_label` (romains `4→iv`, `9→ix`, `14→xiv`, `1994→mcmxciv`, `4000→4000` repli ; alpha `1→a`, `26→z`, `27→aa`, `52→zz`, `53→aaa` ; préfixe ; style `none`), `read_write_round_trip` (arbre à plat et arbre avec `/Kids`), `shift_ranges_deleted_and_inserted`, `write_rejects_bad_order`.
- `links.rs` : `check_uri` (schémas autorisés/refusés, `javascript:` refusé, espace → `%20`, non-ASCII encodé, longueur), `add_link_dict_shape` (clés `Type/Subtype/Rect/Border/F/P/Dest|A` sur un document programmatique ; `/Annots` direct, indirect et absent).
- `pages.rs` : `move_page_bytes_flat_and_nested` (documents lopdf programmatiques : ordre des `/Kids`, `/Count` à tous les niveaux, `/Parent`, attributs hérités copiés quand le parent change) ; `structure.rs` : `mentions`.
- CLI : `parse_view`, `parse_rect`, `parse_label_range` (cas valides, `0`, inversé, style inconnu).

### 8.3 Tests d'intégration (`crates/sunipdf-core/tests/engine.rs`, moteur partagé `engine()`, lecture par PDFium + lopdf, rendu en pixels)

Assertions de pixels : tolérance ±8 par canal, rendu `doc.render_page(i, 1.0)`, pixel `(222,217)` (fixture `bookmarks-links`) ou `(70, H−70)` (fixture imbriquée).

0. **`characterize_pdfium_structure_behaviour`** (étape 0, *avant* tout code de production ; `eprintln!` des observations puis assertions figées sur le comportement réel) : sur `bookmarks-links.pdf` — H7 : `bookmarks()` donne `[Some(0),Some(1),Some(2),Some(3),Some(3),Some(4),None]` ; H1 : après `delete_page(2)` l'élément « Chapitre 1 » a `page == None` ; H2 : après `move_page(0,4)` « Intro » a `page == None` ; H3 : lecture lopdf de l'instantané, les liens de la page déplacée ont-ils encore `/Dest` ? ; H4 : `extract_to(&[0,1])` → `bookmarks()` vide ; H5 : les annotations `Link` de la page extraite ont-elles encore `/Dest` ? (lopdf) ; H6 : le test cherche `/Outlines`, `/PageLabels` et `/Link` en clair dans `doc.snapshot()` (octets) ; H8 : voir test 11. Le résultat décide des étapes 4 et 8 (tableau de décision, section 10).
1. **`bookmark_depths_and_lopdf_alignment`** : `bookmarks()` de `bookmarks-links` → profondeurs `[0,1,0,1,2,1,0]` (régression du correctif 3.1) ; lecture lopdf (`outline::read_items`, exposée `#[doc(hidden)] pub` ou testée en unitaire interne) : mêmes profondeurs et mêmes `Page(i)` que les pages PDFium.
2. **`bookmark_add_rename_delete_round_trip`** : ouvrir une copie ; `edit_bookmarks(Add{After(0)…})`, `Rename`, `SetDestination` (vue `Zoom(1.5)` : relecture lopdf du tableau `/XYZ null null 1.5`), `Delete` d'un parent (sous-arbre) ; après chaque étape `bookmarks()` = liste attendue (titres, pages, profondeurs) ; `save_as` puis réouverture : même liste ; **annulation** : `snapshot()` avant, `restore` après → liste d'origine ; relecture lopdf : `/First /Last /Next /Prev /Parent` cohérents, `/Count` conforme (`3` pour « Chapitre 1 » avant modification), aucun objet de plan orphelin.
3. **`bookmark_move_indent_outdent`** : suites `Up/Down/Indent/Outdent` sur la fixture, comparaison de la liste à plat attendue (dont le cas « les frères suivants restent enfants du parent »).
4. **`bookmark_set_destination_navigates_to_page`** : `SetDestination{index, page: 3}` → `bookmarks()[i].page == Some(3)` **et** la page 3 rendue a la couleur jaune (le lien entre signet et contenu est vérifié par les pixels).
5. **`bookmark_edit_rejects_bad_input`** : `index` hors bornes, titre vide ou blanc, `page` hors bornes → `PageOutOfRange`, `Add` avec profondeur invalide (via `Place`) ; document chiffré (`encrypted-rc4.pdf`, mot de passe `secret`) → `Err(SecurityRestricted)` et document inchangé ; plan dont l'alignement est faux (document construit avec un `/Next` cyclique) → `Other("outline not editable")` ou terminaison sans panique.
6. **`delete_pages_prunes_dead_bookmarks_and_links`** : `bookmarks-links` ; `delete_pages(&[0])` → « Intro » disparaît, « Contexte » remonte à la profondeur 0, aucun élément à `page == None` hors « Annexe » ; `links(0)` (ancienne page 1) : le lien nommé vers l'ancienne page 2 est conservé et vaut `Page{1}` ; `links(2)` (ancienne page 3) : son lien GoTo vers la page supprimée a été retiré (liste vide) ; `delete_pages(&[0,1,2,3,4])` (toutes les pages) → `Err` et document inchangé (contrôle « dernière page ») ; `restore(before)` rétablit tout ; document chiffré à plusieurs pages (produit dans le test par `security::protect` sur la fixture puis `engine.open_bytes(.., Some(pw), ..)`) → suppression réussie, réparation ignorée sans erreur, signets inchangés ; garde : test unitaire de `structure::needs_repair(&[u8], &PageChange)` sur des octets littéraux (sans `/Outlines`, `/PageLabels`, `/Link` → `false`).
7. **`move_page_keeps_bookmarks_links_and_content`** : (a) flat : `move_page(2, 0)` → pages dans l'ordre `[p2,p0,p1,p3,p4]` vérifié par **pixels** (bleu, rouge, vert, jaune, magenta) et par texte ; « Chapitre 1 » (nommé) → `Some(0)`, « Intro » → `Some(1)` ; `links(1)` (ancienne page 0) : lien vers l'ancienne page 2 → `Page{0}` ; (b) `nested-pages.pdf` : `move_page(0, 3)` → ordre `[B,C,D,A]`, la page A déplacée sous le nœud 4 garde sa taille 612×792 (attribut hérité copié) et le rendu garde sa couleur ; `/Count` racine = 4 (lopdf) ; (c) document chiffré → repli sur l'ancien chemin, contenu correct ; (d) tests existants `insert_move_and_snapshot_restore` et `merge_extract_split_round_trip` inchangés et verts.
8. **`extract_and_split_keep_selected_bookmarks`** : `extract_to(&[2,3,4])` → plan `[Chapitre 1(0), Section 1.1(1), Détail(2), Section 1.2(1)]` aux pages `[0,1,1,2]` ; `extract_to(&[1])` → `[Intro(page 0, profondeur 0), Contexte(page 0, profondeur 1)]` (l'ancêtre est conservé et prend la page du descendant) ; page sans signet → aucun plan ; `split(.., 2, ..)` : chaque fichier a un plan cohérent ; le repli « écrire sans plan » en cas d'échec lopdf n'est pas atteignable en intégration : couvert par la revue et par les tests unitaires de `select_for_pages`.
9. **`merge_creates_file_bookmarks`** : `merge([bookmarks-links, two-pages])` → racine « bookmarks-links » (page 0) avec les 7 signets décalés à la profondeur +1, racine « two-pages » (page 5) avec ses 3 signets (pages 5, 5, 6 ; profondeurs 1, 2, 1) ; `merge_with(.., file_bookmarks:false)` → plans sources sans parents ; un seul fichier → pas de parent ; vérification des pages par pixels (page 5 de la fusion = page 0 de `two-pages`, rectangle bleu).
10. **`link_add_list_replace_delete_and_stay_invisible`** : rendu page 0 avant ; `add_link(0, rect, Page{4, Fit})` et `add_link(0, rect2, Uri("https://example.org/x y"))` ; **rendu après identique octet pour octet** (le lien ne dessine rien) ; `links(0)` liste `Page{page:4}` (relu par PDFium, donc la destination écrite par lopdf est bien résolue) et `Uri("https://example.org/x%20y")` ; `replace_link` change la cible en `Page{1}` ; `delete_annotation` le supprime ; `add_link` avec rect < 2 pt, page cible hors bornes, URI `javascript:` → erreurs ; liens de la fixture lus correctement (`Page{2}`, `Uri`, nommé → `Page{2}`, GoTo → `Page{0}`) ; `links()` sur une page sans lien → vide ; le cache de rendu n'est pas concerné (couvert par `State::edit`).
11. **`page_labels_round_trip_and_pdfium_agrees`** : `page_labels()` de la fixture = 2 plages ; `format_label` donne `i, ii, A-1, A-2, A-3` **et** `page_label(i)` (PDFium, `FPDF_GetPageLabel`) renvoie les mêmes chaînes (H8) ; `set_page_labels` avec 3 plages (romain majuscule / décimal préfixé / alpha) puis relecture + PDFium ; `set_page_labels(&[])` retire `/PageLabels` ; erreurs : premier début ≠ 0, ordre non croissant, `first_number == 0` ; décalage : `delete_pages(&[1])` → les débuts des plages suivantes reculent ; `insert_pages_from` avance les débuts.
12. **`structure_edits_are_undoable_in_one_step`** : pour `delete_pages`, `move_page`, `add_link`, `edit_bookmarks`, `set_page_labels` : `snapshot()` → opération → `restore(snapshot)` → `bookmarks()`, `links(0)`, `page_labels()` et la liste des pages identiques à l'état initial (modélise `State::edit`, `worker.rs:372-398`).

### 8.4 Scénarios visuels (`scripts\dev-screenshot.ps1`, paramètres `-Pdf`, `-Commands 'id1,id2'`, `-Drag 'x1,y1,x2,y2;…'`, `-KeysAfter`, `-Out` ; `.\scripts\dev-env.ps1` puis `cargo build -p sunipdf-desktop -j 2` d'abord)

| # | Commande | Ce qu'il faut voir |
|---|---|---|
| S1 | `.\scripts\dev-screenshot.ps1 -Pdf crates\sunipdf-core\tests\fixtures\bookmarks-links.pdf -Commands 'view.bookmarks' -Out s1-panel.png` | 7 signets sur **3 niveaux** (correctif de profondeur), « Annexe » grisé (pas de page), page 1 active |
| S2 | `… -Commands 'view.bookmarks,bookmark.add' -Out s2-add.png` | boîte « Nouveau signet » (icône, titre, sous-titre « page 1 », champ, emplacements, vue) conforme à D020 |
| S3 | `… -Commands 'tool.link' -Drag '420,300,640,340' -Out s3-link.png` | rectangle de glissement puis boîte « Créer un lien » ; variante `… -Drag '420,300,640,340' -KeysAfter '{ENTER}'` : la surimpression montre 3 zones accentuées (2 existantes + la nouvelle) |
| S4 | `… -Commands 'pages.labels' -Out s4-labels.png` | grille de 2 plages (`i`, `A-1`) et aperçu `i, ii` / `A-1, A-2, A-3` |
| S5 | `… -Commands 'view.bookmarks' -KeysAfter '^{DEL}' -Out s5-delete.png` | page 0 supprimée : « Intro » disparaît, « Contexte » remonte au premier niveau, aucun signet mort hormis « Annexe » |
| S6 | thèmes clair/sombre et FR/EN de S2 et S4 | pas de texte tronqué, pas de clé brute affichée |

Le clic droit (menu contextuel) n'est pas pilotable par le script actuel : ajouter (5 lignes) un paramètre `-RightClick "x,y"` utilisant `mouse_event(0x0008/0x0010)` (RIGHTDOWN/RIGHTUP) et `-Click "x,y"`, ou vérifier ce menu à la main (renommer, déplacer, supprimer, annuler).

### 8.5 Commandes de validation (règle de CLAUDE.md, `-j 2` imposé)

```powershell
. .\scripts\dev-env.ps1
cargo fmt --all -- --check
cargo clippy --workspace -j 2 -- -D warnings
cargo test -p sunipdf-core -j 2
cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2
cargo test --workspace -j 2
.\scripts\check-i18n.ps1
```

---

## 9. Nouvelles dépendances

**Aucune.** `lopdf` 0.45 (MIT) est déjà une dépendance de `sunipdf-core` (`Cargo.toml` racine : `lopdf = "0.45"`, `crates/sunipdf-core/Cargo.toml`) ; PDFium et `pdfium-render` 0.9.4 sont déjà en place ; `std::collections::{HashMap, HashSet}` suffisent pour les index. Côté UI, `eframe` a déjà la fonctionnalité `links` (ouverture d'URL, `egui::Context::open_url`). Aucun crate d'expression régulière, d'arbre ou de nombres romains n'est nécessaire (les conversions romain/alphabétique tiennent en ~25 lignes). Aucune mise à jour de `docs/THIRD_PARTY_NOTICES.md`. Pas de nouvelle dépendance de développement : les tests d'intégration utilisent `lopdf` (dépendance normale du crate, donc accessible aux tests d'intégration).

## 10. Risques, pièges, ordre d'implémentation, estimation

### 10.1 Hypothèses PDFium non vérifiables hors ligne et décisions qui en découlent

Les sources de `pdfium-render` sont lues ; le binaire PDFium (build 7881) ne l'est pas. Ces points sont **de mémoire** et tranchés par l'étape 0 (test de caractérisation), qui fige le comportement réel avant d'écrire le code de production.

| # | Hypothèse | Si faux, ou différent |
|---|---|---|
| H1 | `FPDFPage_Delete` retire la page de l'arbre mais laisse l'objet : un signet qui le référence passe à `page == None` (rangée grisée) | `PDFium` retire/retarget lui-même → supprimer la partie « élagage » de `repair_structure`, garder le décalage des étiquettes |
| H2 | Le `move_page` actuel recrée la page : signets et liens entrants deviennent morts | Si faux **et** H3 faux : le déplacement sans perte n'est plus nécessaire → conserver l'ancien chemin et supprimer `move_page_bytes` (−150 lignes) |
| H3 | Les liens *sortants* de la page déplacée perdent leur `/Dest` (import) | idem |
| H4 | `FPDF_ImportPages` n'importe pas le plan (`extract_to`, `merge`, `insert` produisent des documents sans signets) | Si le plan était importé : n'appeler `set_outline` que pour la fusion |
| H5 | À l'import, un tableau de destination qui référence une page est supprimé (la référence vers un objet `/Type /Page` n'est pas clonée) : les liens internes deviennent inertes ; autre possibilité : références orphelines | « supprimé » → étape 8 recrée les liens (lopdf, correspondance ancienne→nouvelle page) ou documente la limite ; « orphelin » → étape 8 élague ; « conservé » → rien |
| H6 | Les instantanés PDFium contiennent les dictionnaires en clair (pas de flux d'objets), donc la garde `mentions(b"/Outlines" …)` n'a pas de faux négatif | Retirer la garde (toujours réparer, plafond 256 Mo) ; le reste de la conception ne dépend pas de la numérotation d'objets |
| H7 | `FPDFBookmark_GetDest` / `FPDFLink_GetDest` résolvent aussi `/A /S /GoTo` et les destinations nommées (la doc de `bindings.rs:5955-5974` dit d'appeler `GetDest` quand `GetAction` est `NULL`) | `Document::bookmarks()` devient hybride (page calculée par la classification lopdf `DestState`) |
| H8 | `FPDF_GetPageLabel` formate comme `format_label` (préfixe + numérotation, romains/alphabétiques) | Ajuster `format_label` sur les fixtures jusqu'à égalité avec PDFium |

### 10.2 Risques techniques et pièges

1. **Fidélité du détour lopdf** : réécriture complète du fichier (flux d'objets dépliés, pas de mise à jour incrémentale) ; un PDF que lopdf ne sait pas lire renvoie `InvalidFormat` (`OperationFailed`, document intact grâce à `State::edit`). Coût O(taille) à chaque édition de structure : mesurer sur un PDF de ~50 Mo (hors thread UI, donc non bloquant) ; plafond `STRUCTURE_REPAIR_MAX_BYTES` pour la réparation automatique.
2. **Chiffrement** : refus explicite (2.2-1). Dette connue à traiter dans une tâche séparée (« conserver le chiffrement après une édition lopdf », qui touche aussi `SetMetadata` et `merge_content_streams`).
3. **Alignement des indices** entre la liste PDFium (affichée) et la lecture lopdf (modifiée) : garde exécutée à chaque `edit_bookmarks` (même nombre, mêmes profondeurs) + test 1 + drapeau `bookmarks_busy` côté UI contre les clics périmés.
4. **`/Count` des signets** : signe et valeur soigneusement recalculés (3.2) ; vérifier à la main dans Edge/Acrobat (le test automatique relit avec PDFium et lopdf, pas avec Acrobat — à faire par l'utilisateur).
5. **Déplacement sans perte** : arbres de pages exotiques (`/Kids` indirect, pages directes dans `/Kids`, `/Parent` manquant) → `Err` → repli sur l'ancienne copie ; le repli ne doit jamais laisser le document modifié à moitié (la mutation lopdf n'est appliquée que par `restore` final).
6. **Lien invisible** : on suppose que PDFium ne dessine rien pour un `Link` sans `/AP` avec `/Border [0 0 0]` ; le test 10 (rendu identique) le prouve ; sinon ajouter `/C []`/`/H /N` ou ajuster.
7. **Coordonnées** : `annotate::to_pdf` ignore `/Rotate` et l'origine de `CropBox` ; les liens dessinés sur une page pivotée seront décalés comme les autres annotations (limite existante, à traiter globalement).
8. **Sécurité des liens** : la création n'accepte que http, https, mailto, tel (`check_uri`) ; **le suivi** n'ouvre que ces mêmes schémas après confirmation (`link_confirm_dialog`) ; `file:`, `javascript:`, `Launch`, `GoToR` ne sont jamais exécutés (listés « Autre »). Aucune requête réseau n'est émise par l'application : l'ouverture est déléguée au navigateur par l'utilisateur.
9. **Conflits de fusion** avec d'autres tranches : `app.rs`, `worker.rs`, `shell.rs`, `command.rs`, `annotate.rs`, `editor.rs` sont des fichiers chauds (vue continue = `document_area` ; édition avancée = `Tool`/`Command`). Mitigation : toute la logique d'affichage de liens est dans `links.rs` sous forme de fonctions prenant `(painter, page_rect, page_index, page_h_pt)` pour être appelées page par page en vue continue ; ajouter les variantes de `Command` et les clés i18n en petits commits dès l'étape 3.
10. **Mémoire** : chaque édition = un instantané complet dans le journal (existant, plafonné à 50 entrées / 512 Mo) + pic transitoire de 3-4 copies.
11. **Texte** : titres non latins encodés en UTF-16BE ; l'affichage dépend de la couverture de Segoe UI (cases vides pour CJK, limite existante).
12. **Tests de clés i18n** : `ui_keys_used_by_app_exist` ne lit que `app.rs`/`main.rs` : étendre (7).
13. **Performance de `mentions`** en profil dev (opt-level 0 pour le crate) : acceptable sur les fixtures ; sur un fichier de 100 Mo prévoir `memchr`-like par `iter().position` sur `b'/'` si mesuré lent.
14. **Raccourcis** : `Ctrl+B` et `K` ne sont pas utilisés ; pendant la saisie dans un champ, seules `Ctrl+O/S/K/F` restent actives (`app.rs:598-632`), donc taper « k » dans un titre ne change pas d'outil.
15. **`Document::delete_object` de lopdf est quadratique** (2.1) : ne jamais l'utiliser dans les boucles de cette tranche.

### 10.3 Ordre d'implémentation — étapes compilables et testables une à une

| Étape | Contenu | Fichiers | Critère de sortie | Taille |
|---|---|---|---|---|
| **0** | Fixtures `bookmarks-links.pdf`, `nested-pages.pdf` + test de caractérisation `characterize_pdfium_structure_behaviour` (aucun code de production) ; consigner les résultats réels H1-H8 dans le plan/KNOWN_ISSUES | `scripts/make_fixtures.py`, `tests/fixtures/*.pdf`, `tests/engine.rs` | `cargo test -p sunipdf-core -j 2` vert ; tableau de décision 10.1 renseigné | S |
| **1** | Correctif de profondeur (`bookmarks_of`), `security::text_string`, `structure::{load_plain, mentions, needs_repair}`, `outline.rs` : types, modèle, opérations à plat, fonctions pures, `select_for_pages`/`rebase`, tests unitaires | `security.rs`, `structure.rs`, `outline.rs`, `lib.rs` | tests unitaires verts ; test 1 (profondeurs) vert | M |
| **2** | Lecture/écriture lopdf du plan, `edit_outline`, `set_outline`, `Document::edit_bookmarks` ; CLI `bookmarks` (+`index`), `bookmark-add/-rename/-remove/-move/-dest` + `parse_view` | `outline.rs`, `CLI/main.rs` | tests 2, 3, 4, 5, 12 (partie signets) verts ; CLI exercée sur la fixture | M |
| **3** | Worker `EditOp::Outline` ; commande `AddBookmark` (+ registre, menus, ToolTab, icône, raccourci) ; panneau (sélection, menu contextuel, renommage, drapeau busy) ; boîte « Nouveau signet » ; clés `bookmarks.*`, `dest.view.*`, `a11y.bookmark_row` ; extension du test de clés | `command.rs`, `worker.rs`, `app.rs`, `shell.rs`, `docinfo.rs`, `icons.rs`, `assets/i18n/*.json` | `cargo test -p sunipdf-i18n -p sunipdf-desktop` ; captures S1, S2 inspectées ; Ctrl+Z annule chaque action | M |
| **4** | `delete_pages` + `repair_structure` (signets ; liens branchés à l'étape 5, étiquettes à l'étape 6) ; `move_page_bytes` + repli ; worker `Delete` ; tests 6, 7 (sans les liens `links()`), unitaires `move_page_bytes` | `pages.rs`, `structure.rs`, `outline.rs`, `worker.rs` | tests existants `rotate_delete_and_save_atomically`, `insert_move_and_snapshot_restore` verts ; S5 | L |
| **5** | `links.rs` moteur (`links`, `add_link`, `replace_link`, `check_uri`, `prune_dead_links` branché dans `repair_structure`) ; worker `Links/AddLink/ReplaceLink` ; UI `links.rs` (outil, surimpression, barre, boîtes, suivi + confirmation) ; `Tool::Link`, `Command::ToolLink` ; CLI `links`, `link-add`, `link-remove` ; clés `links.*`, `a11y.link_area` | `links.rs` (core et desktop), `annotate.rs`, `editor.rs`, `app.rs`, `command.rs`, `shell.rs`, `worker.rs` | test 10 et complément du test 7 ; S3 ; clic sur un lien navigue | L |
| **6** | `labels.rs` (lecture, écriture, `format_label`, `shift_ranges` branché dans `repair_structure` pour `Deleted`/`Inserted`) ; `Document::page_label` ; worker `PageLabels/SetPageLabels` ; commande `PageLabels` + boîte ; CLI `page-labels`, `set-page-labels` ; clés `labels.*` | `labels.rs`, `structure.rs`, `pages.rs`, `docinfo.rs`, `app.rs`, `worker.rs`, `CLI/main.rs` | test 11 ; S4 | M |
| **7** | Conservation des signets à l'extraction/division/fusion : `extract_to`, `Engine::merge_with`, `MergeOptions`, CLI `--no-file-bookmarks` | `pages.rs`, `CLI/main.rs` | tests 8, 9 ; tests existants `merge_extract_split_round_trip` verts | M |
| **8** | *Conditionnelle (H5)* : liens GoTo internes à l'import — recréation (lecture des liens GoTo des pages retenues dans la source par lopdf, correspondance ancien→nouvel index, `add_link` sur la sortie) ou élagage des liens inertes | `links.rs`, `pages.rs` | test dédié selon la décision ; sinon limite documentée | M |
| **9** | Finition : documentation (section 11), `ui_keys…` étendu, clippy/fmt/tests complets, captures S1-S6 inspectées, `KNOWN_ISSUES` | docs, `scripts/dev-screenshot.ps1` (option clic) | commandes de 8.5 vertes avec résultats réels consignés | S |

Chaque étape laisse le workspace compilable ; les étapes 0-3 forment une livraison autonome « édition des signets » (≈ M-L) qui peut être fusionnée avant le reste.

### 10.4 Estimation

**XL** pour la tranche complète (≈ 2 500 à 3 000 lignes dont ~40 % de tests ; 3 nouveaux modules moteur + `structure.rs`, 1 module UI, 5 fichiers UI/worker modifiés, ~74 clés ×2, 10 sous-commandes CLI). Découpage : bloc signets (étapes 0-3) **M-L** ; cohérence (4) **L** ; liens (5) **L** ; étiquettes (6) **M** ; import (7-8) **M-L**.

### 10.5 Index des fichiers

| Action | Fichiers |
|---|---|
| **Créés** | `crates/sunipdf-core/src/{outline,links,labels,structure}.rs`, `apps/sunipdf-desktop/src/links.rs`, `crates/sunipdf-core/tests/fixtures/{bookmarks-links,nested-pages}.pdf` |
| **Modifiés (moteur)** | `lib.rs` (modules + ré-exports), `security.rs` (`bookmarks_of`, `text_string`), `pages.rs` (`delete_pages`, `move_page`, `move_page_bytes`, `extract_to`, `merge_with`, `insert_pages_from`), `command.rs` (3 variantes + entrées), `tests/engine.rs` |
| **Modifiés (application)** | `main.rs` (`mod links;`), `app.rs` (réponses, `execute`, touches, garde des modales, `document_area`, liste des dialogues l. 1876-1881, tests i18n), `worker.rs`, `shell.rs` (ToolTab, icônes, couleurs), `docinfo.rs` (état, panneau, deux dialogues), `annotate.rs` (`Tool::Link`, `AnnotateState.links`, `finish_drag`), `editor.rs` (`page_cursor`), `icons.rs` (`ADD`, `LINK`) |
| **Modifiés (autres)** | `apps/sunipdf-cli/src/main.rs`, `assets/i18n/{fr-FR,en-US}.json`, `scripts/make_fixtures.py`, documentation (section 11) |
| **Intacts** | `tabs.rs`, `dialog.rs`, `tools.rs`, `organize.rs` (le panneau Organiser reste sur `EditOp::Delete/Move`, qui bénéficient automatiquement de la cohérence), `Cargo.toml` (aucune dépendance) |

### 10.6 Points à faire valider par l'utilisateur

1. La fusion crée-t-elle un signet par fichier **par défaut** (comportement proposé, désactivable en CLI uniquement) ?
2. Accepter que l'édition de structure soit refusée sur un document chiffré tant que la conservation du chiffrement n'est pas traitée ?
3. Le clic sur un lien en mode Sélection (nouveau comportement du lecteur) est-il souhaité ?
4. Raccourcis proposés : `Ctrl+B` (signet), `K` (outil lien).
5. Faut-il inclure l'étape 8 (liens GoTo à l'import) si l'étape 0 confirme H5 « supprimé » ?

## 11. Documentation à mettre à jour

| Fichier | Mise à jour |
|---|---|
| `docs/PROJECT_STATUS.md` | nouvelle ligne de tâche (ex. SUNI-040 « Structure : signets, liens, étiquettes ») cochée **uniquement** avec les commandes réellement exécutées et leurs résultats ; nombre de tests mis à jour |
| `docs/NEXT_TASKS.md` | ajouter SUNI-040 (étapes restantes) ; retirer ce qui est livré ; ajouter « conserver le chiffrement après édition lopdf » et « affichage des étiquettes dans les miniatures/le champ de page » |
| `docs/FEATURE_MATRIX.md` | lignes *Signets* (`:24`, édition faite), *Étiquettes de pages* (`:25`), *Liens* (`:41`), *Fusion de N PDF avec signets* (`:50`), *Extraction préservant liens…* (`:52`, limites mesurées à l'étape 0) |
| `docs/ROADMAP.md` | retirer « liens » du reste à faire (`:34`) ; ajouter repli/dépli des signets, destinations nommées, position `XYZ` à la souris, glisser-déposer des signets, déplacement/redimensionnement de liens, étiquettes dans l'UI |
| `docs/KNOWN_ISSUES.md` | nouvelles entrées : *A11* édition de structure refusée sur document chiffré + dette `SetMetadata`/`merge_content_streams` ; *A12* limites signets/liens/étiquettes (2.2) ; comportement PDFium observé (H1-H8) ; coordonnées sur pages pivotées |
| `docs/DECISIONS.md` | **D021** structure (signets, liens, étiquettes) écrite par lopdf sur l'instantané puis rechargée dans PDFium, refus des documents chiffrés ; **D022** déplacement de page sans perte par lopdf avec repli sur la copie PDFium ; **D023** signet par fichier à la fusion, plans conservés à l'extraction |
| `docs/ARCHITECTURE.md` | modules `outline`, `links`, `labels`, `structure` du moteur et `links.rs` de l'application ; schéma « snapshot → lopdf → restore » |
| `docs/I18N.md` | préfixes `bookmarks.dialog.*`, `dest.view.*`, `links.*`, `labels.*`, `a11y.bookmark_row|link_area` |
| `docs/USER_GUIDE.fr.md` / `.en.md` | sections *Signets* (ajout, imbrication, renommage), *Liens* (outil K, suivi), *Étiquettes de pages*, comportement à la suppression/déplacement/fusion/extraction, limites (document chiffré) |
| `CHANGELOG.md` | `[Non publié]` : Ajouté (édition des signets, liens, étiquettes, signet par fichier à la fusion, 10 commandes CLI) ; Corrigé (profondeur des signets plafonnée à 1, liens non cliquables) ; Modifié (déplacement de page sans perte) |
| `CLAUDE.md` | « Modules existants » : ajouter `outline, links, labels` au moteur et `links.rs` à l'application ; « sunipdf-cli (20 commandes JSON) » → 30 |
| `scripts/make_fixtures.py`, `scripts/dev-screenshot.ps1` | nouvelles fixtures ; option `-Click`/`-RightClick` (facultative) |

