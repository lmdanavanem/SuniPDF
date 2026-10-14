# Plan d'implémentation — tranche `edit-advanced` (SUNI-031c, édition du contenu, suite)

> Rédigé le 2026-10-14 par l'agent architecte. **Aucun code ni test n'a été exécuté pour produire ce plan** (contrainte :
> dépôt en lecture seule, pas de `cargo`). Toute affirmation sur le comportement de PDFium est étiquetée :
> **[lu]** = vérifié dans une source réelle (chemin:ligne cité), **[web]** = vérifié dans le code source PDFium
> (`chromium/pdfium`, branche `main`, lu via WebFetch car les sources C++ ne sont pas sur le poste), **[hypothèse]** =
> à confirmer par le test-sonde indiqué *avant* d'écrire l'interface qui en dépend (étape 0).
>
> Abréviations des chemins : `PM` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src`,
> `EG` = `.../egui-0.36.2/src`, `EW` = `.../egui-winit-0.36.2/src`, `PH` = `C:/dev_suniworks/SuniPDF/third_party/pdfium/win-x64/include`
> (en-têtes du binaire PDFium 7881 réellement livré), `CORE` = `crates/sunipdf-core/src`, `DESK` = `apps/sunipdf-desktop/src`.

## 0. Résumé et découvertes à traiter en priorité

Tranche demandée : finir l'éditeur de contenu façon PDFCraft « Edit a PDF » : rotation libre, polices système incorporées,
opacité, panneau Objets, sélection multiple + groupe, couper/copier/coller, édition de texte en place, remplacement d'image,
recadrage (CropBox), redimensionnement de pages, page vierge, et correction des pages tournées.

Découvertes faites pendant la recherche (certaines sont des défauts **déjà présents**, hors de la demande mais bloquants ou voisins) :

| # | Constat | Preuve | Traitement dans ce plan |
|---|---|---|---|
| D1 | **Le code actuel est faux sur les pages `/Rotate 90/180/270` et sur les pages dont la MediaBox/CropBox ne part pas de (0,0).** Les tailles affichées sont celles de la page tournée/rognée, mais toutes les coordonnées d'objets, de texte et d'annotations restent en espace utilisateur PDF non tourné. | `CORE/pages.rs:291-302` (`page_sizes` = tailles *après* rotation, test `rotate_delete_and_save_atomically` : largeur 792) ; `DESK/annotate.rs:99-104` (`to_pdf` ne connaît ni rotation ni origine) ; `CORE/edit.rs:107-140` (`describe` renvoie `FPDFPageObj_GetBounds` brut) ; matrices PDFium **[web]** `core/fpdfapi/page/cpdf_page.cpp` `UpdateDimensions` | §3.1 : l'API du moteur passe en **espace d'affichage** (conversion centralisée ; le mapping écran↔PDF de l'UI ne change pas) |
| D2 | **Ctrl+C (copie de texte) ne se déclenche jamais au clavier** : egui-winit transforme Ctrl+C/X/V en `Event::Copy/Cut/Paste` et **ne pousse pas d'`Event::Key`**. `Event::Paste` n'est même émis que si le presse-papiers système contient du texte. | `EW/lib.rs:1021-1033` et `1418-1440` ; `eframe-0.36.2/src/native/glow_integration.rs:820-832` ; `DESK/app.rs:738` (`key(COMMAND, Key::C, CopyText)`, mort) ; aucun `Event::Copy` dans `apps/` | §5.8 : routage par `Event::Copy/Cut/Paste` + marqueur texte dans le presse-papiers. **À confirmer par capture** (`^c` puis coller dans le Bloc-notes) |
| D3 | `EditState` (sélection, glissement) **n'est pas échangé par onglet** : après un changement d'onglet, la sélection d'un autre document reste affichée. | `DESK/tabs.rs` ne mentionne pas `edit` ; `DESK/app.rs:122` | §5.9 : `edit.reset_for_tab()` dans `take_tab`/`put_tab` ; presse-papiers d'objets **global** |
| D4 | `page_objects()` est **O(n²)** sur les pages denses : `describe` appelle `PdfPageTextObject::text()` qui ouvre (`FPDFText_LoadPage`) une page texte **par objet** ; `object_at` (chaque clic) appelle `page_objects`. | `PM/pdf/document/page/object/text.rs:336-353` ; `CORE/edit.rs:162-195` | §3.2 : une seule page texte (`PdfPageText::for_object`, `PM/pdf/document/page/text.rs:315`) |
| D5 | `duplicate_object` **remonte l'original au premier plan** (effet de bord de `copy_to_page`). | `PM/pdf/document/page/object/group.rs:463-475` (objets déplacés vers une page temporaire) et `535-546` (réajoutés **en fin** de liste) | §3.8 : duplication = copier + coller (sans effet de bord) |
| D6 | `set_object_style` **remet l'opacité à 100 %** (`color(rgb)` force l'alpha à 255). | `CORE/edit.rs:94-96` et `296-322` | §3.4 : l'alpha courant est conservé sauf si `opacity` est fourni |
| D7 | PDFium **n'incorpore jamais de sous-ensemble** : la police entière est copiée dans le PDF. | **[web]** `fpdfsdk/fpdf_edittext.cpp` : `LoadFontDesc` enveloppe le tampon complet (`doc->NewIndirect<CPDF_Stream>(font_data)`), aucune étape de sous-ensemble ; en-tête `PH/fpdf_edit.h:1370-1388` (« data … will be copied by the font object ») | §3.6 : avertissement de poids dans l'UI, plafond de taille, contrôle `fsType` |
| D8 | Le dialogue de texte est multi-ligne (`desired_rows(3)`) mais `add_text` crée **un seul** objet texte contenant des `\n` (comportement des `\n` dans `FPDFText_SetText` **non vérifié**). | `DESK/editor.rs:748-751`, `CORE/edit.rs:196-220` | §3.6 : `add_text` découpe par ligne |
| D9 | Plusieurs autres plans de `docs/plans/` touchent les mêmes fichiers (`tool-hub.md` supprime `ToolTab`/`tool_row` ; `sign-image.md` utilise `Request::ObjectInfo`/`edit.reselect` et note la même limite « pages pivotées » ; `bookmarks-links.md` ajoute `EditOp::AddLink{rect}` qui doit adopter l'espace d'affichage). | `tool-hub.md:30,118,181` ; `sign-image.md:79-82,425-430` ; `bookmarks-links.md:371-373` | §4 (compatibilité des enums) et §10 (coordination) |

**Verdict global : faisable, partiellement** (voir §2) : tout est livrable hors ligne avec des licences permissives et **aucune dépendance nouvelle**, mais
le collage produit un *groupe* non éditable comme texte, les polices sont incorporées en entier, la rotation/redimensionnement de groupe est hors périmètre, et plusieurs points (H2 à H7, §10.1)
reposent sur un comportement PDFium à valider par des tests-sondes (étape 0).

---

## 1. Objectif et comportement visible

Correspondance avec l'outil « Edit a PDF » de PDFCraft (catégorie *Éditer* de SuniPDF).

| # | Ce que voit l'utilisateur | Entrée |
|---|---|---|
| 1 | Sur une page tournée de 90° ou rognée, la sélection d'un objet, les poignées, le texte ajouté, les formes, la surbrillance et la recherche tombent **au bon endroit** ; le texte ajouté apparaît **droit à l'écran**. | outils existants |
| 2 | **Rotation libre** : une poignée ronde au-dessus du cadre de sélection (tige verticale). Glisser = pivoter autour du centre, aperçu du cadre pivoté en direct, `Maj` = pas de 15°, l'angle s'affiche dans la barre d'état. Le champ « Angle (°) » de la barre d'objet applique un angle absolu. Les boutons ±90° restent. | poignée / barre d'objet |
| 3 | **Polices système** : dans l'éditeur de texte, la liste de polices propose « Polices standard » (Helvetica, Times, Courier) puis « Polices du système » (recherche incrémentale, ex. *Segoe UI*, *Arial Bold*). Le texte peut contenir tout caractère de la police (cyrillique, grec, latin étendu). Une note indique que la police est incorporée en entier (poids). Caractère absent ou licence interdisant l'incorporation → message d'erreur précis, rien n'est ajouté. | éditeur de texte |
| 4 | **Opacité** : champ « Opacité (%) » dans la barre d'objet pour textes, formes et images (appliqué avec « Appliquer à l'objet »), et valeur initiale pour les nouvelles formes/images. Rendu réel (transparence PDF `ca`/`CA`). | barre d'objet |
| 5 | **Panneau Objets** (`F5`, bouton du rail droit) : liste des objets de la page courante, du plus en avant au plus en arrière (icône de type, aperçu du texte). Clic = sélectionner (Ctrl+clic = ajouter/retirer), survol = surbrillance sur la page, boutons *avancer/reculer d'un plan* et *supprimer*. | `view.objects` |
| 6 | **Sélection multiple** : rectangle élastique (glisser sur le vide ; `Alt` = objets touchés au lieu de contenus), `Maj+clic`, `Ctrl+A` (tous les objets de la page). **Déplacement groupé** (glisser, flèches), suppression, duplication, premier/arrière-plan, couleur/opacité appliquées à tous. | outil Sélection d'objet |
| 7 | **Couper / copier / coller** (`Ctrl+X/C/V`, menu Édition, barre d'objet) d'objets, **entre pages et entre onglets** ; le collé est un **groupe** (déplaçable, redimensionnable, supprimable ; le texte qu'il contient n'est plus modifiable comme texte). | `edit.copy_objects`… |
| 8 | **Édition de texte en place** : double-clic / `F2` sur un texte, ou clic avec l'outil Texte : un champ de saisie apparaît **sur la page** à la taille du zoom (fond blanc, couleur/taille de l'objet) avec une mini-barre (taille, couleur, police, gras/italique). `Entrée` valide, `Maj+Entrée` = nouvelle ligne, `Échap` annule, clic hors du champ valide. Les textes pivotés (≠ 0° à l'écran) gardent le dialogue actuel. | `F2` / double-clic |
| 9 | **Remplacer l'image** : l'image sélectionnée est remplacée par un autre fichier, **même cadre** (proportions du nouveau fichier conservées dans le cadre si l'objet n'est pas pivoté). | menu / barre d'objet |
| 10 | **Recadrer** (marges en mm, aperçu en direct sur la page, page courante ou toutes, « Rétablir le cadre d'origine »), **Redimensionner** (A3/A4/A5/Letter/Legal/personnalisé, paysage, contenu conservé ou mis à l'échelle), **Insérer une page vierge** (taille, nombre, position). Menu *Pages*, onglet d'outils *Pages*, palette Ctrl+K. | `pages.crop`, `pages.resize`, `pages.insert_blank` |

Tout est annulable (instantanés existants) et enregistré dans le contenu du PDF (pas de calque).

---

## 2. Faisabilité : verdict **partiel** (oui sur tout, avec les limites honnêtes suivantes à documenter)

| Élément | Verdict | Limite assumée (à écrire dans `KNOWN_ISSUES.md` / guides) |
|---|---|---|
| Pages tournées / rognées | **oui** | Dépend de l'hypothèse H2 (rectangles de texte PDFium en espace utilisateur) — test-sonde étape 0. |
| Rotation libre | **oui** | Pivot = centre des limites ; le cadre orienté n'est exact que pour texte et image (`FPDFPageObj_GetRotatedBounds`, `PH/fpdf_edit.h:987-1000`) ; pour un tracé pivoté on dessine la boîte englobante. Redimensionner un objet pivoté d'un angle ≠ multiple de 90° est **refusé** (poignées de coin masquées). |
| Polices système | **oui, TrueType (`.ttf`) seulement** | `.otf` à contours PostScript (en-tête `OTTO`), collections `.ttc`, polices variables (instance par défaut seulement) : **hors périmètre** (erreur `FontNotEmbeddable`). **Pas de sous-ensemble** (D7) : +0,4 à 1,6 Mo par police et par document. Contrôle de la licence `fsType` (refus si bit 0x0002/0x0200 ou « aperçu/impression » seul). Valider par `qpdf --check` et un autre lecteur (Edge) : **non vérifié ici**. |
| Opacité | **oui** pour texte, tracés, images | Formes/XObjects collés : non garanti (contrôle désactivé pour `Group`/`Other`). **[web]** `core/fpdfapi/edit/cpdf_pagecontentgenerator.cpp` : `ProcessImage` appelle `ProcessGraphics`, qui écrit l'ExtGState `ca/CA/BM` quand l'alpha ≠ 1. |
| Panneau Objets | **oui** | Réordonnancement par boutons ▲▼ (pas de glisser-déposer). |
| Sélection multiple, déplacement groupé | **oui** | **Rotation et redimensionnement de groupe : hors périmètre** (feuille de route). |
| Couper/copier/coller | **oui, collé = groupe** | Le collé est un objet *formulaire* (XObject) : non éditable comme texte, couleur non modifiable (comme `duplicate_object` aujourd'hui, `KNOWN_ISSUES` A8). Collage « natif éditable » : feuille de route. Fidélité visuelle exacte, polices/images conservées. Le groupe est rogné à la boîte de la page source. |
| Édition en place | **oui, avec police-proxy** | Le champ utilise la police de l'interface (Segoe UI / monospace), pas la police PDF : largeur et retours à la ligne approximatifs pendant la saisie. Pas d'édition de texte pivoté ni de paragraphes réorganisés. |
| Remplacer l'image | **oui** | L'image est ré-encodée en bitmap BGRA (`FPDFImageObj_SetBitmap`) : un JPEG devient plus lourd ; l'ancienne image reste orpheline jusqu'à « Optimiser ». |
| Recadrer | **oui** | **Masque seulement** : le contenu hors cadre reste dans le fichier (message dans le dialogue ; ce n'est **pas** un caviardage — relève de `security-signatures`). Recadrage par dessin à la souris : feuille de route (v1 = marges numériques avec aperçu). |
| Redimensionner | **oui, mode « conserver » sûr ; mode « adapter » au mieux** | Le mode « adapter » (`FPDFPage_TransFormWithClip`) **ne déplace pas** les annotations/liens existants ; comportement à valider par le test de l'étape 0 (H4). |
| Page vierge | **oui** | — |

Hors périmètre explicite (à inscrire en `ROADMAP.md`) : sous-ensemble de polices (crate permissive `subsetter` + post-traitement lopdf, ADR à écrire), polices `.otf`/`.ttc`, collage éditable, rotation/redim. de groupe,
recadrage à la souris, édition de paragraphes, modes de fusion (l'API `set_blend_mode` existe, `PM/pdf/document/page/object.rs:719`).

---

## 3. API moteur (`sunipdf-core`)

### 3.0 Principes (valables pour tout le §3)

1. **Espace d'affichage.** Toute coordonnée qui franchit la frontière moteur ↔ interface/CLI est en *espace d'affichage* : origine en bas à gauche de la page **telle qu'affichée**
   (après `/Rotate`, relative à la boîte effective = MediaBox ∩ CropBox), axe Y vers le haut, unité = point. C'est exactement ce que `Document::page_size` renvoie déjà et ce que
   `annotate::to_pdf` suppose (`DESK/annotate.rs:99-104`, `DESK/app.rs:1081-1082`). L'UI n'a donc **aucun changement de mapping** ; seules des fonctions du moteur convertissent,
   à un seul endroit (`PageGeometry`). Sur une page non tournée dont la boîte part de (0,0), la conversion est l'identité : **aucun test existant ne change de résultat**.
   Règle à imposer aux autres plans (`bookmarks-links` `add_link`, `sign-image`) : tout nouveau `PointRect` reçu de l'UI est en espace d'affichage.
2. **Régénération du contenu.** Chaque mutation d'objet se termine par `page.regenerate_content()?` (`PM/pdf/document/page.rs:942`) : les setters de couleur ne régénèrent pas
   (`PM/pdf/document/page/object.rs:923-937` n'appelle pas `regenerate_content_after_mutation`), et `edit.rs` le fait déjà partout. Une fonction multi-objets n'appelle `regenerate_content` qu'**une fois** à la fin.
3. **Instantané / annulation.** Les fonctions du moteur ne gèrent pas l'annulation : le worker appelle `Document::snapshot()` avant `apply`, restaure en cas d'erreur et empile l'instantané en cas de succès
   (`DESK/worker.rs:372-398`). Toute fonction qui appelle `restore()` en interne (`merge_content_streams`, `edit.rs:402-422`) invalide les `PdfPage` déjà ouverts : **réobtenir la page après**.
4. **Erreurs typées.** Deux variantes nouvelles (`MissingGlyphs`, `FontNotEmbeddable`, §3.11) ; le reste réutilise `EngineError::Other(String)` avec des messages anglais stables (testés par `matches!`/`is_err`).

### 3.1 Géométrie de page — nouveau module `CORE/geometry.rs`

Formules (page `bbox = (l, b, r, t)` = `FPDF_GetPageBoundingBox`, rotation `R` horaire ; `(x,y)` utilisateur, `(X,Y)` affichage). **[web]** `core/fpdfapi/page/cpdf_page.cpp::UpdateDimensions`
donne `page_matrix_` = `CFX_Matrix(a,b,c,d,e,f)` avec `x' = a·x + c·y + e`, `y' = b·x + d·y + f` :
R0 `(1,0,0,1,-l,-b)` ; R90 `(0,-1,1,0,-b,r)` ; R180 `(-1,0,0,-1,r,t)` ; R270 `(0,1,-1,0,t,-l)` (largeur/hauteur échangées pour R90/R270), d'où :

| R | `to_display(x,y)` → (X,Y) | `to_user(X,Y)` → (x,y) | taille d'affichage |
|---|---|---|---|
| 0 | `(x-l, y-b)` | `(X+l, Y+b)` | `(W,H)` |
| 90 | `(y-b, r-x)` | `(r-Y, X+b)` | `(H,W)` |
| 180 | `(r-x, t-y)` | `(r-X, t-Y)` | `(W,H)` |
| 270 | `(t-y, x-l)` | `(Y+l, t-X)` | `(H,W)` |

(`W = r-l`, `H = t-b`). Contrôle de cohérence par les coins : sous R90 le coin utilisateur haut-gauche `(l,t)` devient `(H, W)` (haut-droit à l'écran) ; sous R270 le coin bas-gauche `(l,b)` devient `(H, 0)` (bas-droit).

```rust
// CORE/geometry.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry { pub bbox: PointRect, pub rotation: Rotation }

impl PageGeometry {
    pub fn display_size(&self) -> (f32, f32);
    pub fn to_display(&self, x: f32, y: f32) -> (f32, f32);
    pub fn to_user(&self, x: f32, y: f32) -> (f32, f32);
    pub fn rect_to_display(&self, r: PointRect) -> PointRect;   // convertit 2 coins puis normalise (min/max)
    pub fn rect_to_user(&self, r: PointRect) -> PointRect;
    /// Partie linéaire seule (vecteurs : déplacements). R0 (dx,dy) ; R90 (-dy,dx) ; R180 (-dx,-dy) ; R270 (dy,-dx).
    pub fn vec_to_user(&self, dx: f32, dy: f32) -> (f32, f32);
    pub fn vec_to_display(&self, dx: f32, dy: f32) -> (f32, f32);
    /// Matrice PDF `[a,b,c,d,e,f]` qui place l'origine locale d'un objet en (x,y) d'affichage avec ses axes alignés
    /// sur ceux de l'écran (« droit »). = `[vec_to_user(1,0), vec_to_user(0,1), to_user(x,y)]`.
    pub fn upright_matrix(&self, x: f32, y: f32) -> [f32; 6];
    /// Rotation `/Rotate` en degrés horaires (0/90/180/270).
    pub fn rotation_degrees(&self) -> f32;
}

pub(crate) fn geometry_of(page: &PdfPage<'_>) -> Result<PageGeometry, EngineError> {
    let b = page.boundaries().bounding()?.bounds;               // PM boundaries.rs:248-263 -> FPDF_GetPageBoundingBox (PH/fpdfview.h:764-775)
    let rotation = Rotation::from_pdfium(page.rotation()?);     // PM page.rs:292 ; rendre `Rotation::from_pdfium` pub(crate) (pages.rs:52)
    Ok(PageGeometry { bbox: PointRect { left: b.left().value, bottom: b.bottom().value, right: b.right().value, top: b.top().value }, rotation })
}

impl Document<'_> {
    pub fn page_geometry(&self, page_index: usize) -> Result<PageGeometry, EngineError> {
        geometry_of(&self.page_handle(page_index)?)
    }
}
```

`lib.rs` : `pub mod geometry;` + `pub use geometry::PageGeometry;`. `Rotation::{to_pdfium, from_pdfium}` passent de `fn` privée à `pub(crate) fn` (`CORE/pages.rs:43-60`).
`bounding()` est documentée « enveloppe du contenu » par pdfium-render mais appelle bien `FPDF_GetPageBoundingBox` (= MediaBox ∩ CropBox, **[lu]** `PH/fpdfview.h:764-775`) : citer ce point en commentaire.

Tests unitaires purs (dans `geometry.rs`, sans PDFium) : aller-retour `to_user(to_display(p)) == p` pour les 4 rotations et une `bbox` décalée `(100,50,712,842)` ; coins ci-dessus ; `upright_matrix` appliquée à `(1,0)` et `(0,1)` redonne `vec_to_user`.

### 3.2 Description des objets — `CORE/edit.rs`

```rust
pub enum ObjectKind { Text, Image, Path, Form /* nouveau : XObject formulaire (copies/collages) */, Other }

pub struct ObjectInfo {
    pub index: usize,
    pub kind: ObjectKind,
    pub bounds: PointRect,                // ESPACE D'AFFICHAGE (boîte englobante des 4 sommets de `quad`)
    pub quad: [(f32, f32); 4],            // NOUVEAU : sommets du cadre orienté, espace d'affichage (ordre non garanti : l'UI trie par angle)
    pub angle_deg: f32,                   // NOUVEAU : rotation affichée, horaire, dans (-180, 180]
    pub opacity: f32,                     // NOUVEAU : 0..1 (alpha de remplissage ; min(remplissage, trait) pour un tracé)
    pub text: Option<String>, pub font_size: f32, pub font_name: Option<String>,
    pub fill_rgb: Option<(u8,u8,u8)>, pub stroke_rgb: Option<(u8,u8,u8)>, pub stroke_width: f32,
}
```

`describe(index, object, geo, text_page)` (signature privée) :
- `kind` : ajouter `PdfPageObjectType::XObjectForm => ObjectKind::Form` (`PM/pdf/document/page/object.rs:76-100`, enum `PdfPageObjectType`).
- `quad` : `object.bounds()` renvoie un `PdfQuadPoints` (`PM/.../object/private.rs:321-372`) : **quadrilatère serré pour texte et image** (`FPDFPageObj_GetRotatedBounds`), rectangle pour le reste (`FPDFPageObj_GetBounds`). Lire `x1()..y4()` (`PM/pdf/quad_points.rs:151-193`), convertir chaque sommet par `geo.to_display`. **Ne pas** utiliser `quad_to_rect` (min/max) pour ça : `bounds` = min/max des sommets *affichage*.
- `angle_deg` : `object.get_rotation_clockwise_degrees()` (`PM/pdf/transform.rs:370` = `-atan2(b,a)` de `FPDFPageObj_GetMatrix`, `private.rs:399-419`; getters disponibles sur l'enum `PdfPageObject`, `object.rs:650-656`) `+ geo.rotation_degrees()`, normalisé. Pour un texte ajouté « droit » sur une page R90, la matrice utilisateur vaut −90 et l'angle affiché 0.
- `opacity` : `object.fill_color().ok().map(|c| c.alpha() as f32 / 255.0)` (`object.rs:892-921`) ; pour `Path`, `min` avec l'alpha de `stroke_color()` (`object.rs:940-969`) ; `1.0` si lecture impossible.
- `text` : `text_page.for_object(t)` (**une** page texte ouverte par `page_objects`, `PM/pdf/document/page/text.rs:315-354`) au lieu de `t.text()` (D4). `object_info(index)` ouvre sa propre page texte.

Fonctions de lecture :
- `page_objects(page)` : `let geo = geometry_of(&page)?; let text = page.text().ok();` puis `describe` pour chaque objet (O(n)).
- `object_at(page, x, y)` : parcourt `(0..n).rev()` en ne calculant que `quad`/`bounds` (pas de texte), teste le point par **polygone** si `|angle| > 0,5°` (sinon rectangle avec tolérance 1 pt comme aujourd'hui), puis `describe` **le seul** objet touché.
- `objects_in_rect(page, rect, mode: RectMode) -> Vec<ObjectInfo>` avec `enum RectMode { Contained, Touching }` (Contained : les 4 sommets dans `rect` ; Touching : intersection des `bounds`). L'UI utilise `Contained` (un scan pleine page n'est ainsi pas capté par un rectangle élastique), `Alt` = `Touching`.

### 3.3 Conversions dans les fonctions existantes (comportement inchangé en rotation 0 / origine 0)

| Fonction (fichier) | Entrée UI → utilisateur | Sortie → affichage |
|---|---|---|
| `add_text(page,x,y,…)` (`edit.rs:196`) | matrice `geo.upright_matrix(x, y)` appliquée par `object.transform(a..f)` (`PM/pdf/transform.rs:68`) **au lieu de** `translate(x,y)` : le texte est posé *droit à l'écran* sur toute page | — |
| `add_image(page,x,y,w)` (`edit.rs:463`) | taille `w × h` par `scale(w,h)` (`transform.rs:146`) puis `transform(upright_matrix(x,y))` : l'image est droite à l'écran, coin bas-gauche d'affichage en (x,y) | — |
| `add_shape(page,kind,rect,style)` (`edit.rs:490`) | `Rectangle`/`Ellipse` : `geo.rect_to_user(rect)` ; `Line` : `to_user` sur chaque extrémité | — |
| `move_object(dx,dy)` (`edit.rs:246`) | `geo.vec_to_user(dx,dy)` | — |
| `resize_object(bounds)` (`edit.rs:267`) | `rect_to_user(bounds)` **et** `rect_to_user(bounds courants)`, puis l'algorithme existant (translate/scale/translate) en espace utilisateur | — |
| `rotate_object(deg)` (`edit.rs:355`) | pivot = `geo.to_user(centre des bounds d'affichage)` ; angle horaire inchangé (une rotation d'écran horaire est une rotation utilisateur horaire) | — |
| `text_in_rect(page,rect)` (`annot.rs:70`) | `rect_to_user(rect)` | `rects` → `rect_to_display` (H2) |
| `search_page` (`text.rs:31`) | — | rectangles → `rect_to_display` (H2) |
| `add_markup(page,kind,rects,rgb)` (`annot.rs:116`) | chaque rect → `rect_to_user` (les points d'ancrage `PdfQuadPoints` restent alignés aux axes utilisateur) | — |
| `add_note(page,x,y,text)` (`annot.rs:159`) | rectangle d'icône construit en affichage (x, y-20)→(x+20, y) puis `rect_to_user` | — |
| `annotations(page)` (`annot.rs:178`) | — | `rect` → `rect_to_display` |
| `stamp_pages` (`stamp.rs:96`) | aucun changement de code : n'utilise que `page_size`, `add_text`, `object_info`, `move_object`, `rotate_object` (tous en affichage) | — |

Note : sur page tournée, `rotate_object(page, i, -stamp.angle)` après `add_text` droit donne le même rendu qu'en rotation 0 (le tampon est visuellement identique).

### 3.4 Opacité

```rust
pub struct ObjectStyle { pub fill_rgb: Option<(u8,u8,u8)>, pub stroke_rgb: Option<(u8,u8,u8)>, pub stroke_width: Option<f32>,
                         pub opacity: Option<f32> /* NOUVEAU, 0..1, bornée */ }   // Default dérivé : les littéraux existants (`..Default::default()`) compilent
pub struct ShapeStyle { /* existant */ pub opacity: f32 /* NOUVEAU, défaut 1.0 */ }
```
- `set_object_style` (`edit.rs:296`) : lire `fill_color()`/`stroke_color()` courants ; si `fill_rgb` **ou** `opacity` est fourni → `set_fill_color(PdfColor::new(r,g,b, alpha))` avec `alpha = opacity.map(alpha_u8).unwrap_or(alpha courant)` (corrige D6) ; idem `set_stroke_color` pour les `Path` seulement ; texte : remplissage seul ; image/autres : remplissage seul (la couleur est ignorée au rendu, seul l'alpha compte).
  Appels : `PdfPageObjectCommon::set_fill_color` → `FPDFPageObj_SetFillColor(R,G,B,A)` (`PM/.../object.rs:923-937`, **[lu]** `PH/fpdf_edit.h:1100-1114`) ; **[web]** PDFium stocke l'alpha dans `general_state().SetFillAlpha(A/255.f)` et `ProcessGraphics` écrit `ca`/`CA` (§2).
- `add_shape` : `color_a(rgb, alpha)` au lieu de `color(rgb)` ; les constructeurs `create_path_object_*` appliquent les `PdfColor` avec alpha (`PM/.../object/path.rs:236-257`).
- `add_image_with(page, path, x, y, width_pt, opacity)` : nouvelle fonction ; `add_image(...)` conservée (délègue avec `1.0`, les tests existants compilent). Après `PdfPageImageObject::new_with_size`/`transform`, `object.set_fill_color(PdfColor::new(255,255,255,alpha))` avant `add_image_object`.
- `add_text` : `TextStyle.opacity` existe déjà et fonctionne (test `stamp_watermark_and_page_numbers`).
- Corriger les 3 littéraux `ShapeStyle { … }` des tests existants (`tests/engine.rs:712, 797, 912`) avec `..Default::default()`.

### 3.5 Rotation libre

Aucun changement de signature : `rotate_object(page, index, degrees: f32)` accepte déjà n'importe quel angle fini (`edit.rs:355-378`), pivot au centre des limites, `NaN` rejeté. Les ajouts sont **lecture** (`angle_deg`, `quad`, §3.2) et **UI** (§5.2). Dérive de pivot : le centre d'un rectangle/ellipse/ligne/texte/image est invariant par rotation ; pour un tracé asymétrique le centre des `bounds` change à chaque rotation successive → dérive possible (limite documentée, `ponytail:` dans le code).
Angle absolu depuis le champ numérique : l'UI calcule `delta = cible - info.angle_deg` et envoie `EditOp::RotateObject{degrees: delta}`.

### 3.6 Polices système — nouveau module `CORE/fonts.rs` et `add_text`

**Ce que fait vraiment PDFium (D7, [web] `fpdfsdk/fpdf_edittext.cpp` + en-tête `PH/fpdf_edit.h:1370-1388`)** — `FPDFText_LoadFont(doc, data, size, font_type, cid)` :
- `font_type = FPDF_FONT_TRUETYPE` (`PH/fpdf_edit.h:55-56`) ; les données sont **copiées** ; **aucun sous-ensemble** : le flux `FontFile2` contient le fichier entier.
- `cid = false` → `LoadSimpleFont` : police TrueType *simple*, codes 1 octet (`GetCharCodesAndIndices(0xFF)`), `FirstChar/LastChar/Widths`, **pas d'`/Encoding`** : couverture limitée au Latin-1, et portabilité douteuse des accents hors PDFium. **À ne pas utiliser.**
- `cid = true` → `LoadCompositeFont` : `Type0`/`Identity-H`, descendant `CIDFontType2`, tableau `/W` par indice de glyphe, **`/ToUnicode` généré pour tous les glyphes** (`LoadUnicode`) ; `/CIDToGIDMap` absent (= Identity par défaut pour du TrueType incorporé, donc code = indice de glyphe). Couverture = tous les glyphes de la police. **Choix retenu : `is_cid_font = true`.**
- Conversion texte → codes : `FPDFText_SetText` appelle `CharCodeFromUnicode(wc)` pour chaque caractère ; pour un caractère absent la valeur retournée est `0` (glyphe `.notdef`) — **[web]** `core/fpdfapi/font/cpdf_cidfont.cpp` (aucun contrôle de présence dans la police) → un caractère manquant n'est **pas** une erreur PDFium ; il faut le détecter nous-mêmes (H3).
- pdfium-render : `PdfFonts::load_true_type_from_bytes(&[u8], is_cid_font: bool) -> Result<PdfFontToken, PdfiumError>` (`PM/pdf/document/fonts.rs:531-537`) → `new_font_from_bytes` → `FPDFText_LoadFont` (`fonts.rs:538-568`) ; `PdfFontToken` est `Copy + Hash` (`fonts.rs:85`) ; l'ensemble des polices appartient au `PdfDocument` (`PdfDocument::fonts_mut`, `PM/pdf/document.rs:290`) et est fermé avec lui. `PdfPageTextObject::new(document, text, font: impl ToPdfFontToken, size)` (`PM/.../object/text.rs:197-210`).

**Énumération des polices installées, sans crate** :

```rust
// CORE/fonts.rs
#[derive(Debug, Clone, PartialEq)]
pub struct SystemFont { pub name: String /* « Arial Bold » */, pub path: PathBuf, pub size_bytes: u64 }

/// Polices `.ttf` installées, triées par nom, dédoublonnées. Jamais d'erreur : liste vide en cas d'échec.
/// Source : `reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts"` puis HKCU (polices par utilisateur,
/// chemins absolus) via `std::process::Command` + `CREATE_NO_WINDOW` (0x08000000, `std::os::windows::process::CommandExt`).
/// Repli si `reg.exe` est indisponible : `std::fs::read_dir(%WINDIR%\Fonts)` filtré sur `.ttf`, nom = nom de fichier.
pub fn system_fonts() -> Vec<SystemFont>;

/// Pur, testable : lignes `    Arial Bold (TrueType)    REG_SZ    arialbd.ttf` (CRLF) -> (nom sans « (TrueType) », fichier).
pub(crate) fn parse_reg_output(output: &str) -> Vec<(String, String)>;

/// Contrôle avant incorporation : en-tête sfnt `00 01 00 00` ou `true` (sinon `OTTO` = PostScript, `ttcf` = collection) ;
/// table `OS/2` -> `fsType` (u16 gros-boutiste à l'offset 8 de la table). Refus si (fsType & 0x0002) ou (fsType & 0x0200)
/// ou ((fsType & 0x000E) == 0x0004) [aperçu/impression seul]. Taille <= 32 Mio. Pas de table OS/2 -> autorisé.
pub(crate) fn check_embeddable(bytes: &[u8]) -> Result<(), EngineError>;   // Err(FontNotEmbeddable)
```
Format de sortie `reg query` **vérifié sur ce poste** (CRLF, 4 espaces, `REG_SZ`) : `    Arial (TrueType)    REG_SZ    arial.ttf`. Parsing : `line.split_once("REG_SZ")`, nom = partie gauche `trim()` sans le suffixe ` (TrueType)`/` (OpenType)`, fichier = partie droite `trim()` ; ignorer toute entrée dont l'extension n'est pas `.ttf` ; fichier relatif -> `%WINDIR%\Fonts\<fichier>` ; abandonner l'entrée si `!path.is_file()` (couvre les noms non ASCII mal décodés : `reg.exe` écrit en page de code OEM, décodage `String::from_utf8_lossy`). Cette machine : 353 entrées HKLM, 324 `.ttf` (147 Mo), 15 `.ttc` (exclues).
Alternatives écartées : crate `windows` (déjà dans l'arbre, `Cargo.lock` `windows 0.62.2`, mais feature `Win32_System_Registry` à activer = recompilation lourde sur ce poste à 14 Go) ; `skrifa 0.44.0`/`read-fonts` (présents via `epaint`, MIT/Apache-2.0) pour lire noms/cmap — **inutile** : les noms viennent du registre et la couverture se vérifie par aller-retour PDFium (ci-dessous). Chemin d'évolution : `windows` + `RegEnumValueW`.

**Chargement et cache par document** :

```rust
// Document gagne un champ privé :  font_cache: HashMap<PathBuf, PdfFontToken>   (vidé dans `restore()` et `restore_with_password()` car `self.inner` est remplacé)
impl Document<'_> {
    fn font_token_for_file(&mut self, path: &Path) -> Result<PdfFontToken, EngineError> {
        if let Some(t) = self.font_cache.get(path) { return Ok(*t); }          // évite une 2e copie complète de la police dans le PDF
        let bytes = std::fs::read(path)?;                                       // EngineError::Io
        check_embeddable(&bytes)?;                                              // FontNotEmbeddable
        let token = self.inner.fonts_mut().load_true_type_from_bytes(&bytes, true)?;
        self.font_cache.insert(path.to_path_buf(), token);
        Ok(token)
    }
}
```
Piège : sans le cache, chaque `add_text` ré-incorpore la police (≈ 1 Mo). Après un Annuler/Rétablir (`restore`), le cache est vidé : la 1re insertion suivante ré-incorpore ; les doublons orphelins sont retirés par « Optimiser » (documenter).

**`add_text` révisé** (`TextStyle` gagne `pub font_file: Option<PathBuf>` ; `Default` = `None` ; les littéraux existants qui n'utilisent pas `..Default::default()` — `DESK/editor.rs:844`, `DESK/tools.rs:294,303`, `apps/sunipdf-cli/src/main.rs:360,389`, `CORE/stamp.rs:63,82` — ajoutent le champ ou `..Default::default()`) :

```rust
pub fn add_text(&mut self, page_index: usize, x: f32, y: f32, text: &str, style: &TextStyle) -> Result<usize, EngineError> {
    // validations existantes (texte non vide, taille finie > 0) ; `Result<usize,_>` = nombre d'objets ajoutés (compatible avec `.unwrap();`)
    let font = match &style.font_file { Some(p) => self.font_token_for_file(p)?, None => font_token(self.inner.fonts_mut(), style) };
    let geo = self.page_geometry(page_index)?;
    let pitch = style.size_pt * 1.2;
    let mut page = self.page_handle(page_index)?;
    let mut added = 0;
    for (i, line) in text.split('\n').map(|l| l.trim_end_matches('\r')).enumerate() {
        if line.trim().is_empty() { continue; }                                  // ligne vide = interligne
        let mut object = PdfPageTextObject::new(&self.inner, line, font, PdfPoints::new(style.size_pt))?;
        object.set_fill_color(PdfColor::new(r, g, b, alpha))?;
        let m = geo.upright_matrix(x, y - i as f32 * pitch);
        object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
        let mut o = page.objects_mut().add_text_object(object)?;
        if style.font_file.is_some() {
            // H3 : lecture inverse par PDFium (objet rattaché à la page, `text()` exige une page : text.rs:336-353)
            let back = o.as_text_object().map(|t| t.text()).unwrap_or_default();
            if back != line {                                                    // caractère absent de la police -> .notdef
                for _ in 0..=added { let n = page.objects().len(); page.objects_mut().remove_object_at_index(n - 1)?; }
                return Err(EngineError::MissingGlyphs);                          // le document est laissé intact
            }
        }
        added += 1;
    }
    page.regenerate_content()?;
    Ok(added)
}
```
`set_object_text` n'est **pas** modifié (pas de contrôle de présence de glyphes : risque de faux refus sur les polices d'origine sans `ToUnicode`, voir `KNOWN_ISSUES` A6). Le dialogue/éditeur en place le signale par la note existante.
`font_family`/`bold`/`italic` sont ignorés quand `font_file` est présent (la variante est choisie dans la liste : « Arial Bold »).

### 3.7 Opérations multi-objets (une seule entrée d'annulation par geste)

```rust
pub enum ZOrder { Front, Back, Forward, Backward }
impl Document<'_> {
    pub fn move_objects(&mut self, page: usize, indices: &[usize], dx: f32, dy: f32) -> Result<(), EngineError>;   // un regenerate_content
    pub fn delete_objects(&mut self, page: usize, indices: &[usize]) -> Result<(), EngineError>;                   // dédoublonne, supprime par index décroissant, un regenerate
    pub fn set_objects_style(&mut self, page: usize, indices: &[usize], style: &ObjectStyle) -> Result<(), EngineError>;
    /// Renvoie les nouveaux index (dans l'ordre de `indices` trié croissant).
    pub fn reorder_objects(&mut self, page: usize, indices: &[usize], op: ZOrder) -> Result<Vec<usize>, EngineError>;
    pub fn objects_in_rect(&self, page: usize, rect: PointRect, mode: RectMode) -> Result<Vec<ObjectInfo>, EngineError>;
    pub fn duplicate_objects(&mut self, page: usize, indices: &[usize], dx: f32, dy: f32) -> Result<usize, EngineError>;  // = copy_objects + paste_objects (§3.8)
}
pub(crate) fn reorder_plan(count: usize, selected: &[usize], op: ZOrder) -> Vec<(usize, usize)>;   // PUR : suite de (from, to) « remove(from) puis insert(to) »
```
- Validation commune : liste non vide, index `< count` sinon `Other("object N not found")`, déplacement fini.
- `reorder_plan` (pure, testée sans PDFium par simulation sur un `Vec<usize>` d'identifiants) : **Front** — pour le k-ième index trié croissant `i` : `(i-k, count-1)` ; **Back** — pour le k-ième index trié **décroissant** : `(i+k, 0)` ; **Forward/Backward** — un cran, en parcourant du côté de la destination et en sautant un objet bloqué par un voisin sélectionné ou par la fin. L'ordre relatif des objets sélectionnés est conservé.
- Exécution : `Front` = `remove_object_at_index` puis `add_object` (comme `reorder_object(front=true)`, `edit.rs:424`) ; les autres cas appellent d'abord `merge_content_streams` (fusion lopdf, `edit.rs:402`, car PDFium dessine flux par flux) puis `remove_object_at_index` + `insert_object_at_index(to, obj)` (`PM/pdf/document/page/objects.rs:150-156`, FFI `FPDFPage_InsertObjectAtIndex`, **[lu]** `PH/fpdf_edit.h:210`). Les fonctions mono-objet existantes (`move_object`, `delete_object`, `reorder_object`) restent (tests existants) et délèguent.
- `duplicate_object` (mono) devient `copy_objects` + `paste_objects` : supprime D5 ; l'objet dupliqué reste un `Form` comme aujourd'hui (test existant `edit_rotate_duplicate_reorder` : décalage (+12,−12) des limites à reconfirmer).

### 3.8 Presse-papiers d'objets — nouveau module `CORE/clipboard.rs`

Principe : un clip est **un PDF d'une page en mémoire** contenant uniquement les objets copiés (polices, images et ressources comprises) ; le collage importe cette page comme **XObject formulaire** dans la page cible, y compris dans **un autre document** (onglet différent). Aucun état PDFium n'est conservé entre copie et collage (pas de `Document` clip vivant, donc pas de souci de durée de vie ni de fermeture).

```rust
#[derive(Debug, Clone)]
pub struct ObjectClip {
    pdf: Vec<u8>,                      // PDF d'une page (privé)
    pub count: usize,                  // nombre d'objets copiés
    pub text: String,                  // texte des objets texte, séparés par '\n' (marqueur pour le presse-papiers système)
    pub source_page: usize,
    pub source_rotation: Rotation,
    pub bounds: PointRect,             // union des limites d'affichage dans la page source
}
impl Document<'_> {
    /// N'altère PAS le document (contrairement à `copy_to_page`, cf. D5).
    pub fn copy_objects(&self, page: usize, indices: &[usize]) -> Result<ObjectClip, EngineError>;
}
impl<'a> Document<'a> {
    /// Colle en un seul objet `Form` ; (dx,dy) = décalage en points d'affichage par rapport à la position d'origine ;
    /// renvoie l'index du groupe collé. `'a` identique pour `self` et le document temporaire : voir risque R5.
    pub fn paste_objects(&mut self, page: usize, clip: &ObjectClip, dx: f32, dy: f32) -> Result<usize, EngineError>;
}
```
`copy_objects` :
1. valider ; calculer `bounds`/`text`/`source_rotation` depuis `describe` (limites d'affichage) ;
2. `let mut tmp = self.pdfium().create_new_pdf()?;` (`PM/pdfium.rs:500`) ; `tmp.pages_mut().copy_page_from_document(&self.inner, page_index, 0)?;` (`PM/pdf/document/pages.rs:262-274`, même motif que `move_page`, `CORE/pages.rs:219-245`) — copie la page avec ses ressources, sans toucher à l'original ;
3. `let mut p = tmp.pages().get(0)?;` supprimer **tous les objets non sélectionnés** par index décroissant (`remove_object_at_index`, `PM/pdf/document/page/objects/common.rs:368,664`), `p.regenerate_content()?`, relâcher `p` ;
4. `pdf = tmp.save_to_bytes()?` (`PM/pdf/document.rs:388`).
`paste_objects` :
1. `let src = self.pdfium().load_pdf_from_byte_vec(clip.pdf.clone(), None)?;` (`PM/pdfium.rs:330`) ; `let sp = src.pages().get(0)?;` (met la page dans le cache d'index, `pages.rs:124-142`, requis par l'étape suivante) ;
2. `let form = sp.objects().copy_into_x_object_form_object(&mut self.inner)?;` (`PM/pdf/document/page/objects.rs:94-132` → `FPDF_NewXObjectFromPage(dest_doc, src_doc, 0)` + `FPDF_NewFormObjectFromXObject`, **[lu]** `PH/fpdf_ppo.h:80-101` : « template … for use in |dest_doc| » = import inter-documents) ;
3. `let mut page = self.page_handle(page)?; let mut obj = page.objects_mut().add_object(form)?;` (`common.rs:118,396`) ;
4. **compensation de rotation de page** : l'objet est posé en espace utilisateur ; pour conserver l'aspect à l'écran : `obj.rotate_clockwise_degrees(src_rot - dst_rot)` autour du centre de ses limites (via les mêmes translate/rotate/translate que `rotate_object`) ;
5. **placement** : lire les limites d'affichage actuelles `ob` du collé (après compensation) ; `delta = (clip.bounds.left + dx − ob.left, clip.bounds.bottom + dy − ob.bottom)` en espace d'affichage ; `obj.translate(vec_to_user(delta))` ; si le rectangle obtenu ne coupe pas la page cible, le recentrer sur la page ;
6. `page.regenerate_content()?` ; renvoyer l'index (dernier objet).
Limites : le groupe est borné à la boîte de la page source (BBox du XObject = boîte de page) ; objets hors page source perdus ; `describe` renvoie `kind = Form`. Hypothèse H5 : `FPDFPageObj_GetBounds` d'un formulaire est serré (union de ses objets) — **déjà impliquée** par le test existant `edit_rotate_duplicate_reorder` (décalage (+12,−12) des limites de la copie).

### 3.9 Remplacer une image

```rust
pub fn replace_image(&mut self, page: usize, index: usize, path: &Path) -> Result<(), EngineError>;
```
1. `image::open(path)` (erreur `Other("<chemin>: …")` comme `add_image`) ; refuser une image de 0 pixel ;
2. `let mut page = self.page_handle(page)?; let mut o = page.objects().get(index)?;` `let Some(img_obj) = o.as_image_object_mut() else { Err(Other("not an image object")) }` (`PM/.../object.rs:483`) ;
3. mémoriser `bounds`/`angle` d'affichage ; `img_obj.set_image(&dyn_image)?` (`PM/.../object/image.rs:686-722` → `PdfBitmap` BGRA puis `FPDFImageObj_SetBitmap(null, 0, obj, bitmap)`, `image.rs:724-738`, **[lu]** `PH/fpdf_edit.h:793-805`) : **la matrice de l'objet est conservée**, donc l'image remplie le même cadre (étirée si le ratio diffère) ;
4. si `|angle| < 0,5°` : recalculer un cadre « contenir » (ratio du nouveau fichier dans les anciens `bounds`, centré) et l'appliquer par `resize_object` ; sinon laisser étiré ;
5. `page.regenerate_content()?`.
Pièges : image ré-encodée en BGRA (poids) ; l'ancien flux image reste orphelin ; passer `null/0` pour les pages chargées (comme `add_image`).

### 3.10 Pages : recadrer, redimensionner, insérer — nouveau module `CORE/boxes.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq)] pub struct PageMargins { pub left: f32, pub top: f32, pub right: f32, pub bottom: f32 }   // points, espace d'affichage
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ResizeMode { KeepContent /* ancré en haut à gauche d'affichage */, ScaleContent /* ajusté, centré */ }
impl Document<'_> {
    pub fn crop_page(&mut self, page: usize, m: PageMargins) -> Result<(f32, f32), EngineError>;     // nouvelle taille d'affichage
    pub fn reset_crop(&mut self, page: usize) -> Result<(), EngineError>;                            // CropBox := MediaBox
    pub fn resize_page(&mut self, page: usize, width_pt: f32, height_pt: f32, mode: ResizeMode) -> Result<(), EngineError>;
    pub fn insert_blank_pages(&mut self, at: usize, count: usize, width_pt: f32, height_pt: f32) -> Result<(), EngineError>;
}
```
- **`crop_page`** : `geo = geometry_of(&page)`, taille d'affichage `(Wd,Hd)` ; rectangle d'affichage restant `N = (m.left, m.bottom, Wd - m.right, Hd - m.top)` ; refuser une marge négative/non finie ou une zone `< 18 pt` de côté (`Other("crop area too small")`) ; `U = geo.rect_to_user(N)` ; `page.boundaries_mut().set_crop(PdfRect::new(bottom,left,top,right))` (`PM/.../page/boundaries.rs:229-241`, ordre `PdfRect::new(bottom, left, top, right)`, `PM/pdf/rect.rs:68` ; FFI **[lu]** `PH/fpdf_transformpage.h:30-37`) puis `refresh_page_sizes()` (`pages.rs:291`). H7 : `FPDF_GetPageSizeByIndexF` voit le changement sans rechargement (le test existant de rotation s'appuie sur le même mécanisme).
- **`reset_crop`** : `let media = page.boundaries().media()?.bounds; set_crop(media)` (`boundaries.rs:113-121`, `229`).
- **`resize_page`** : cible utilisateur `(uw,uh) = (w,h)` ou échangé si R90/R270. `KeepContent` : nouvelle boîte d'affichage ancrée en haut à gauche — rectangle d'affichage `(0, Hd - h, w, Hd)` converti par `rect_to_user` — appliquée à **MediaBox et CropBox** (`set_media` `boundaries.rs:122-134`, `set_crop`). `ScaleContent` : après avoir posé les nouvelles boîtes `(0,0,uw,uh)`, calculer `s = min(w/Wd, h/Hd)`, `T` = (échelle `s`, centrage `((w-s·Wd)/2, (h-s·Hd)/2)`) en espace d'affichage, matrice utilisateur `M = geo_new.to_user ∘ T ∘ geo_old.to_display` (images de `(0,0)`, `(1,0)`, `(0,1)`), puis `page.transform_with_clip(a,b,c,d,e,f, PdfRect(nouvelle boîte))` (`PM/pdf/document/page.rs:749-762` → `FPDFPage_TransFormWithClip`, **[lu]** `PH/fpdf_transformpage.h:162-178` ; la page est rechargée en interne, `page.rs:764-790`). H4 : l'ordre « boîtes d'abord, transformation ensuite » est requis parce que PDFium intersecte le rectangle de clip avec la boîte courante. Annotations/liens **non transformés** (note permanente dans le dialogue).
- **`insert_blank_pages`** : `at <= page_count` sinon `PageOutOfRange` ; `w,h` finis et `> 0` (borne 14 400 pt = 200 pouces) ; boucle `self.inner.pages_mut().create_page_at_index(PdfPagePaperSize::new_custom(PdfPoints::new(w), PdfPoints::new(h)), (at + k) as PdfPageIndex)?` (`PM/pdf/document/pages.rs:232-257`, `PM/pdf/document/page/size.rs:425`) ; relâcher chaque `PdfPage` ; `refresh_page_sizes()`.
- `refresh_page_sizes` : rendre `pub(crate)` (privée aujourd'hui, `pages.rs:291`).

### 3.11 Erreurs (`CORE/error.rs`)

```rust
#[error("font does not contain every requested character")]  MissingGlyphs,       // code "missing_glyphs"
#[error("font cannot be embedded (licence, format or size)")] FontNotEmbeddable,  // code "font_not_embeddable"
```
À propager : `EngineError::code()`, `DESK/app.rs:1924-1936` (`error_key` : `error.missing_glyphs`, `error.font_not_embeddable`), le tableau du test `every_error_key_exists_in_both_catalogs` (`app.rs:1943-1960`), CLI (`exit code` 1 par défaut, `apps/sunipdf-cli/src/main.rs:655-668`).

### 3.12 Vérification des appels de bibliothèque (tous lus dans les sources réelles)

| Appel | Source (ligne) |
|---|---|
| `PdfPage::boundaries()/boundaries_mut()`, `rotation()`, `regenerate_content()`, `transform_with_clip()`, `apply_matrix_with_clip()`, `text()`, `delete()` | `PM/pdf/document/page.rs:401/407, 292, 942, 749, 764, 377, 861` |
| `bounding()` (= `FPDF_GetPageBoundingBox`), `media()`, `set_media()`, `crop()`, `set_crop()` | `PM/pdf/document/page/boundaries.rs:248-263, 113, 122, 220, 229` |
| `PdfRect::new(bottom,left,top,right)`, `PdfMatrix::new(a..f)`, `PdfPagePaperSize::new_custom` | `PM/pdf/rect.rs:68`, `PM/pdf/matrix.rs:52`, `PM/pdf/document/page/size.rs:425` |
| `PdfQuadPoints::x1()…y4()` | `PM/pdf/quad_points.rs:151-193` |
| `PdfPageObject::bounds()` (quad tendu texte/image) | `PM/.../object/private.rs:321-372` |
| `transform`, `translate`, `scale`, `rotate_clockwise_degrees`, `matrix`, `get_rotation_clockwise_degrees` | `PM/pdf/transform.rs:68, 136, 146, 207, 299, 370` (instanciés pour l'enum `PdfPageObject`, `object.rs:639-656`) |
| `fill_color/set_fill_color/stroke_color/set_stroke_color` (alpha inclus) | `PM/.../object.rs:892, 923, 940, 971` |
| `PdfPagePathObject::new_*` applique couleurs + alpha | `PM/.../object/path.rs:211-261` |
| `PdfPageTextObject::new/font/text/set_text/scaled_font_size/set_unscaled_font_size` | `PM/.../object/text.rs:197, 308, 336, 406, 267, 295` |
| `PdfPageText::for_object` | `PM/pdf/document/page/text.rs:315` |
| `PdfPageImageObject::new/new_with_size/set_image/set_bitmap/get_raw_image` | `PM/.../object/image.rs:107, 259, 686, 724, 288` |
| `PdfPageObjects::insert_object_at_index` / `copy_into_x_object_form_object` / `create_empty_group` | `PM/.../page/objects.rs:150, 94, 88` |
| `add_object`, `remove_object_at_index` (retourne l'objet retiré) | `PM/.../page/objects/common.rs:118/396, 368/664` |
| `PdfPageGroupObject::copy_to_page` (déplace temporairement les objets, D5) | `PM/.../object/group.rs:400, 428-545` |
| `PdfPages::get/page_sizes/create_page_at_index/copy_page_from_document` | `PM/pdf/document/pages.rs:124, 180, 232, 262` |
| `Pdfium::create_new_pdf`, `load_pdf_from_byte_vec` ; `PdfDocument::fonts_mut/pages_mut/save_to_bytes` | `PM/pdfium.rs:500, 330` ; `PM/pdf/document.rs:290, 308, 388` |
| `PdfFonts::load_true_type_from_bytes` → `FPDFText_LoadFont` ; `PdfFontToken: Copy` | `PM/pdf/document/fonts.rs:531-568, 85` |
| En-têtes PDFium 7881 : `FPDFText_LoadFont` (cid, copie des données), `FPDFPageObj_SetFillColor`, `FPDFImageObj_SetBitmap`, `FPDFPageObj_GetRotatedBounds`, `FPDF_GetPageBoundingBox`, `FPDFPage_SetCropBox/SetMediaBox`, `FPDFPage_TransFormWithClip`, `FPDF_NewXObjectFromPage` | `PH/fpdf_edit.h:1388, 1114, 802, 999`, `PH/fpdfview.h:774`, `PH/fpdf_transformpage.h:24, 37, 176`, `PH/fpdf_ppo.h:86, 101` |

---

## 4. Worker (`DESK/worker.rs`) — seul fil qui touche PDFium

Règle de compatibilité : **on ajoute, on ne retire pas** ce dont d'autres plans dépendent (`Request::ObjectAt`, `Request::ObjectInfo` avec la sentinelle `usize::MAX` = dernier objet, `Response::ObjectSelected`, `edit.reselect`) : `sign-image.md:425-430` les utilise tels quels.

### 4.1 Types ajoutés

```rust
pub enum Pick { At { x: f32, y: f32 }, InRect { rect: PointRect, touching: bool }, Indices(Vec<usize>), Last(usize), All }
pub enum PickMode { Replace, Toggle, Add }          // fusion faite par l'UI (le worker renvoie toujours les objets décrits)

// Request (ajouts)
Pick { page: usize, pick: Pick, mode: PickMode },   // sélection (clic, élastique, Ctrl+A, resélection après opération)
Objects { page: usize },                            // liste du panneau Objets
CopyObjects { page: usize, indices: Vec<usize> },   // copie (aucun instantané, document inchangé)
SystemFonts,                                        // mis en cache dans State

// Response (ajouts)
ObjectsSelected { page: usize, infos: Vec<ObjectInfo>, mode: PickMode },
Objects { page: usize, list: Vec<ObjectInfo> },
Copied { count: usize, text: String },              // `text` = texte des objets texte (marqueur du presse-papiers système)
SystemFonts(Arc<Vec<SystemFont>>),
```

`State` gagne `clip: Option<ObjectClip>` et `fonts: Option<Arc<Vec<SystemFont>>>`. **`clip` n'est effacé ni par `Open`, ni par `Close`, ni par `Suspend`/`ResumeDoc`** (les arms `Open`…`Close` de `run`, `worker.rs:662-815`, ne le touchent pas aujourd'hui : ne pas l'ajouter à `state.undo.clear()`), ce qui rend le collage possible d'un onglet à l'autre.
Arme de `run` :
- `Pick` : `At` → `doc.object_at` ; `InRect` → `doc.objects_in_rect(page, rect, if touching {Touching} else {Contained})` ; `Indices(v)` → `object_info` pour chaque index valide (les index hors bornes sont ignorés, pas d'erreur) ; `Last(n)` → `object_info` des `n` derniers index ; `All` → `objects_in_rect(page, page_complete, Touching)`.
- `Objects` → `doc.page_objects(page)` (O(n) grâce à D4).
- `CopyObjects` → `state.clip = Some(doc.copy_objects(page, &indices)?)` puis `Response::Copied{count, text}` ; erreur → `OperationFailed`.
- `SystemFonts` → `state.fonts.get_or_insert_with(|| Arc::new(sunipdf_core::fonts::system_fonts()))`.

### 4.1 bis Points de branchement dans `worker.rs`

`EditOp` : `worker.rs:76` ; `Request` : `178` ; `Response` : `238` ; `State` : `314` (+ `clip`, `fonts`) ; `State::edit` : `372` ; `apply` : `415` (arms des `EditOp`) ; boucle `run` : `618` (arms des `Request` à partir de `662`, `Request::Edit(op) => state.edit(op)` en `717`, `Request::ObjectAt` en `770` — y ajouter les nouveaux arms `Pick`, `Objects`, `CopyObjects`, `SystemFonts`).
**`BatchOp` : aucune variante** (`worker.rs:31`) : aucune de ces opérations ne produit un nouveau fichier ni n'est non annulable ; les sorties de la CLI passent par `Document::save_as`. Les mutations de pages (`CropPages`, `ResizePages`, `InsertBlankPages`) sont des `EditOp` annulables, comme `Rotate`/`Delete` (`worker.rs:416-442`).

### 4.2 `EditOp` (modifications)

| Variante | Changement |
|---|---|
| `MoveObject` → **`MoveObjects { page, indices: Vec<usize>, dx, dy }`** | remplace ; appelle `doc.move_objects` |
| `DeleteObject` → **`DeleteObjects { page, indices }`** | remplace ; `doc.delete_objects` |
| `ReorderObject` → **`ReorderObjects { page, indices, op: ZOrder }`** | remplace ; `doc.reorder_objects(..).map(\|_\| ())` |
| `DuplicateObject` → **`DuplicateObjects { page, indices }`** | remplace ; `doc.duplicate_objects(page, &indices, 12.0, -12.0)` |
| `SetObjectStyle { page, index, style }` → **`{ page, indices: Vec<usize>, style }`** | `doc.set_objects_style` ; `ObjectStyle.opacity` |
| `SetText { page, index, text, size, rgb }` | inchangé (le bloc `rgb` appelle `set_object_style` avec `indices: [index]`) |
| `AddText { page, x, y, text, style }` | inchangé ; `style.font_file` ; retour `Result<usize,_>` ignoré par `.map(\|_\| ())` |
| `AddImage { …, width_pt }` | **ajout** du champ `opacity: f32` → `doc.add_image_with(.., opacity)` |
| **`PasteObjects { page, dx, dy }`** | nouveau ; `clip` fourni par `apply` : `let c = clip.as_ref().ok_or(Other("empty clipboard"))?; doc.paste_objects(page, c, dx, dy).map(\|_\| ())` |
| **`ReplaceImage { page, index, path }`** | nouveau ; `doc.replace_image` |
| **`CropPages { pages, margins }`**, **`ResetCrop { pages }`**, **`ResizePages { pages, width_pt, height_pt, mode }`**, **`InsertBlankPages { at, count, width_pt, height_pt }`** | nouveaux ; boucle sur `pages` (triées, dédoublonnées, validées) ; `Insert…` : `doc.insert_blank_pages` |

*Couper* n'est **pas** une variante : l'UI envoie `Request::CopyObjects` puis `Request::Edit(EditOp::DeleteObjects)` ; le canal du worker étant FIFO, la copie précède la suppression, et Annuler ne défait que la suppression.
`apply` change de signature : `fn apply(doc: &mut Document<'_>, op: EditOp, clip: &Option<ObjectClip>) -> Result<(), EngineError>` ; `State::edit` l'appelle avec `&self.clip` (emprunts disjoints de `self.document` et `self.clip`, pas d'appel de méthode de `self` entre les deux, comme dans le code actuel `worker.rs:372-398`).
Autres plans qui ajoutent des arms à `apply` (`bookmarks-links`, `sign-image`) : rebaser, aucun conflit sémantique.

### 4.3 Réponses et resélection

- Après une opération, l'UI renseigne `edit.reselect` (inchangé : `Option<(page, index)>` pour un objet, `usize::MAX` = dernier) **ou** `edit.reselect_many: Option<(page, Pick)>` (nouveau) ; `DocumentChanged` envoie `Request::Pick{ mode: Replace }` si `reselect_many`, sinon `Request::ObjectInfo` comme aujourd'hui (`app.rs:446-451`).
- Résélections : déplacement/style = `Pick::Indices(mêmes)` ; Premier plan = `Pick::Last(n)` ; Arrière-plan = `Pick::Indices(0..n)` ; avancer/reculer d'un cran = `Indices([i±1])` ; coller et dupliquer un groupe = `Pick::Last(1)` ; ajout de texte = `Pick::Last(nombre de lignes non vides)` ; remplacement d'image = `Indices([i])` ; suppression/coupe = aucune.

---

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Fichiers

| Fichier | Nature | Contenu |
|---|---|---|
| `editor.rs` | modifié (≈ +350 lignes) | `Selection`, `DragMode::{Rotate, Marquee}`, poignée de rotation, sélection élastique, barre d'objet (opacité, angle, presse-papiers, remplacer l'image), routage presse-papiers |
| `objects.rs` | **nouveau** | panneau Objets |
| `inline_text.rs` | **nouveau** | éditeur de texte en place + sélecteur de police partagé avec le dialogue |
| `pagesetup.rs` | **nouveau** | dialogues Recadrer / Redimensionner / Page vierge + aperçu du recadrage |
| `app.rs` | modifié | `handle_input` (§5.8), `execute` (9 arms), `handle_responses` (4 réponses), `document_area` (appels `inline_text_editor`, `paint_crop_preview`), `ui()` (panneau + dialogues), garde des dialogues |
| `worker.rs` | modifié | §4 |
| `shell.rs` | modifié | listes d'outils, `command_icon` (match exhaustif), `command_color`, bouton du rail, activation Copier/Couper/Coller |
| `tabs.rs` | modifié | `edit.reset_for_tab()` (D3) |
| `icons.rs` | modifié | glyphes (à vérifier par capture, repli texte déjà prévu) : `CROP "\u{E7A8}"`, `PASTE "\u{E77F}"`, `CUT "\u{E8C6}"`, `SELECT_ALL "\u{E8B3}"`, `ADD "\u{E710}"`, `LAYERS "\u{E81E}"` |
| `main.rs` | modifié | `mod objects; mod inline_text; mod pagesetup;` |
| `annotate.rs` | **inchangé** pour le mapping (`to_pdf` reste valable : le moteur parle en espace d'affichage) |

### 5.2 Rotation libre (poignée)

- `ObjectInfo` fournit `quad`, `angle_deg`, `bounds`. Cadre dessiné = polygone des 4 sommets (triés par angle autour du centre : l'ordre renvoyé par PDFium n'est pas garanti) pour `Text`/`Image` ; boîte englobante pour les autres types.
- Position de la poignée (pt, espace d'affichage) :
```rust
const ROTATE_GAP_PX: f32 = 26.0;
fn rotate_handle(info: &ObjectInfo, k: f32) -> (f32, f32) {
    let (cx, cy) = center(info.bounds);
    let a = if matches!(info.kind, ObjectKind::Text | ObjectKind::Image) { info.angle_deg.to_radians() } else { 0.0 };
    let up = (a.sin(), a.cos());                               // « haut » local, Y vers le haut, angle horaire
    let half = info.quad.iter().map(|p| (p.0 - cx) * up.0 + (p.1 - cy) * up.1).fold(0.0, f32::max);
    let r = half + (HANDLE_OFFSET_PX + ROTATE_GAP_PX) / k;
    (cx + up.0 * r, cy + up.1 * r)
}
fn pointer_angle(c: (f32, f32), p: (f32, f32)) -> f32 { (p.0 - c.0).atan2(p.1 - c.1).to_degrees() }   // horaire depuis le haut
```
- `DragMode::Rotate { c: (f32,f32), theta0: f32, base: f32 }` pris dans `edit_drag_start` si le point de pression est à moins de `HANDLE_PX` de la poignée (testé **avant** les coins). `theta0 = pointer_angle(c, drag.start)`.
- Pendant le glissement : `delta = norm180(pointer_angle(c, drag.current) - theta0)` ; `Maj` : `delta = round((base + delta) / 15) * 15 - base`. Aperçu : sommets pivotés horaires autour de `c` : `x' = cx + dx·cosδ + dy·sinδ`, `y' = cy − dx·sinδ + dy·cosδ` (Y vers le haut), tige + disque de la poignée, `status_msg = status.rotating`.
- Relâchement : si `|delta| > 0,25°` → `EditOp::RotateObject { page, index, degrees: delta }` (le moteur pivote autour du centre de `bounds`, identique à `c`) ; `reselect` sur le même index.
- Poignées de coin **masquées** quand l'objet n'est pas redimensionnable : `angle_deg` non multiple de 90° à 0,5° près (redimensionner un objet pivoté déformerait). Sélection multiple : ni poignée de rotation ni poignées de coin, cadre englobant en pointillés, déplacement seul.
- Curseurs : survol poignée = `CursorIcon::Grab`, glissement = `Grabbing`.
- Champ « Angle » de la barre d'objet : `DragValue` −180..180, applique à `drag_stopped() || lost_focus()` avec `delta = cible − info.angle_deg` (jamais à chaque image).

### 5.3 Modèle de sélection, rectangle élastique, groupe

```rust
pub struct Selection { pub page: usize, pub objects: Vec<ObjectInfo> }          // triés par index
impl Selection { pub fn single(&self) -> Option<&ObjectInfo>; pub fn union_bounds(&self) -> PointRect; pub fn indices(&self) -> Vec<usize>; }
// EditState : `selected: Option<(usize, ObjectInfo)>` est REMPLACÉ par `selection: Option<Selection>`.
// Couche de compatibilité pour limiter le diff : `selected_on(page) -> Option<&ObjectInfo>` ne renvoie Some que pour une sélection d'UN objet,
// donc `corner_at`, `preview_bounds` (redimensionnement), `open_text_prompt_for_selection` (editor.rs:175-330) restent valables.
```
Réponses : `Response::ObjectSelected{info}` (un objet) → `Selection` de 1 ; `ObjectsSelected{infos, mode}` → `Replace` : remplace ; `Add` : union par index ; `Toggle` : bascule chaque index. `status.objects_selected` si plusieurs.
`edit_drag_start(page, pt)` (ordre de test) : poignée de rotation → `Rotate` ; coin (sélection unique redimensionnable) → `Resize` ; point dans le cadre englobant de la sélection → `Move` ; sinon `Marquee` (même au-dessus d'un objet non sélectionné ; pour déplacer un objet non sélectionné : le sélectionner d'abord, comportement actuel).
`edit_drag_finished` pour `Marquee` : rectangle minuscule → `edit_click(start, shift)` ; sinon `Request::Pick{ InRect{ rect, touching: alt }, mode: if shift {Add} else {Replace} }`.
`edit_click(page, pt)` : `Maj` → `Pick{ At, Toggle }` ; sinon comportement actuel (`ObjectAt`, déjà sélectionné = rien).
Flèches : `nudge_selected` envoie `MoveObjects` avec **tous** les index. `Suppr`, `Ctrl+Maj+D`, `Ctrl+]`, `Ctrl+[` agissent sur tous (`DeleteObjects`, `DuplicateObjects`, `ReorderObjects{Front|Back}`). Ctrl+A (`Tool::SelectObject`) → `Pick{ All, Replace }`.
Aperçu du déplacement groupé : chaque objet de la sélection est tracé décalé de `(dx,dy)` + cadre englobant.

### 5.4 Panneau Objets (`objects.rs`)

- État dans `EditState` : `objects_open: bool`, `objects: Vec<ObjectInfo>`, `objects_page: Option<usize>`, `hover_index: Option<usize>`. Persisté dans `settings` ? **non** (état de session ; seul `show_tools` l'est).
- `refresh_objects_if_needed()` (appelée comme `refresh_annotations_if_needed`, `annotate.rs:291-301`, au début de `document_area`) : si ouvert et `objects_page != Some(doc.page)` → `Request::Objects{page}` ; `DocumentChanged`, changement d'onglet et changement de page remettent `objects_page = None`.
- Panneau : `egui::Panel::right("objects")`, `default_size(260.0)`, cadre `p.chrome`, même schéma que `comments_panel` (`annotate.rs:303-383`). Ligne = `Frame` surlignée si sélectionnée : icône de type (`glyph::FONT/PICTURE/RECT/DUPLICATE/DOCUMENT`), libellé (texte tronqué à 40 caractères, sinon `object.kind.*`), boutons `objects.forward` / `objects.backward` (envoient `ReorderObjects{ indices:[i], op: Forward|Backward }`, resélection `Indices([i±1])`) et `objects.delete` (`DeleteObjects`). Ordre d'affichage : index décroissant (devant en haut).
- Clic ligne → `Request::Pick{ Indices([i]), Replace }` ; `Ctrl` → `Toggle` ; `Maj` → `Add`. Survol → `hover_index` → `paint_object_selection` trace un liseré fin sur la page.
- Entrée dans `ui()` : `if self.document.is_some() && self.edit.objects_open && !self.organize.active { Panel::right("objects")… }` juste après le panneau Commentaires ; bouton dans `right_rail` après « Commentaires » (`glyph::LAYERS`, `.selected(self.edit.objects_open)`).

### 5.5 Barre d'objet (`editor.rs::object_bar`, 609-710)

Ajouts (même `Frame`) : `DragValue` « Opacité » (0..=100, suffixe ` %`, désactivé pour `Form`/`Other`), `DragValue` « Angle » (objet unique), boutons icône Copier / Couper / Coller (activés si sélection / si presse-papiers non vide), « Remplacer l'image » (objet `Image` unique), libellé `bar.group` pour plusieurs objets.
Opacité : `edit.opacity_pct` est synchronisé avec `info.opacity` à la sélection ; hors sélection il désigne le défaut des nouvelles formes/images. « Appliquer à l'objet » envoie `opacity: Some(pct/100)` seulement si `opacity_dirty` (éviter de réécrire l'alpha en changeant une couleur). Nouvelles formes : `ShapeStyle.opacity`, nouvelles images : `EditOp::AddImage{opacity}`.

### 5.6 Édition de texte en place (`inline_text.rs`)

- `TextPrompt` (editor.rs:20-33) gagne : `inline: bool`, `font: TextFont` (`enum TextFont { Standard(FontFamily), System(SystemFont) }`), `armed: bool`. `prompt_is_inline(info)` = objet texte avec `|angle_deg| < 0,5°` ; un nouveau texte (outil Texte) est toujours en place (le moteur le pose droit même sur page tournée). Sinon on garde `text_prompt_dialog` (D8 : conservé, utilise désormais le même sélecteur de police).
- `pub fn inline_text_editor(&mut self, ctx, page_rect: egui::Rect, page_index: usize, page_h_pt: f32)` est appelée **dans la fermeture du `ScrollArea` de `document_area`**, après `paint_object_selection` (besoin de `page_rect`). Avec la future vue continue (`continuous-view.md:542-546`) elle s'appelle par page-fente, comme `paint_matches`.
- Position écran : `page_rect.min + vec2(x·k, (page_h_pt − top)·k)` avec `k = effective_zoom · SCREEN_PER_PDF_POINT` ; pour un nouveau texte `top = y + 0,9·taille` (la ligne de base est au clic). Police : `FontId::new(taille·k, Monospace si Courier sinon Proportional)` (Segoe UI est en tête de `Proportional`, `icons.rs:84-90`).
- Widget : `Area::new(Id::new("inline-text")).order(Order::Foreground).fixed_pos(pos)` contenant un `Frame` fond blanc / liseré accent et `TextEdit::multiline(&mut prompt.text).font(..).text_color(..).frame(Frame::NONE).desired_width(max(120, largeur estimée)).desired_rows(1).return_key(Some(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)))` (`EG/widgets/text_edit/builder.rs:306,321,422` : `Maj+Entrée` = saut de ligne, `Entrée` seul n'est plus consommé donc détectable). Mini-barre dans une seconde `Area` 34 px au-dessus : taille (`DragValue`), couleur (`color_edit_button_srgb`), sélecteur de police, gras/italique (désactivés pour une police système, `text.font.style_note`), note de poids (`text.font.embed_note`), aide (`text.inline.hint`).
- Validation : `Entrée` sans `Maj`, ou clic hors des deux rectangles si `!egui::Popup::is_any_open(ctx)` (`EG/containers/popup.rs:674` : évite de valider quand on utilise la liste de polices ou la pipette) et `armed` (le clic qui crée l'éditeur est ignoré : `armed = true` après la 1re image). `Échap` annule. La validation réutilise le bloc « submit » extrait de `text_prompt_dialog` (`editor.rs:811-858`) → fonction `commit_text_prompt(&mut self, submit: bool)`. `edit_drag_finished` pour `AddText` : si `text_prompt.take().is_some()` (clic de validation) ne pas ouvrir un nouvel éditeur.
- Échec `MissingGlyphs` : le dernier prompt validé est conservé dans `edit.last_prompt` ; sur `Response::OperationFailed(MissingGlyphs | FontNotEmbeddable)` il est rouvert pour correction (bandeau d'erreur `error.*`).
- Sélecteur de police (`font_picker(ui, &mut TextFont, &mut filter, fonts: &Option<Arc<Vec<SystemFont>>>)`) : `ComboBox` ; recherche (`TextEdit::singleline` + `text.font.search`) ; groupes `text.font.standard` (3 familles) et `text.font.system` (filtrage insensible à la casse) ; `Request::SystemFonts` envoyée **une fois** à la première ouverture (`text.font.loading` tant que `None`) ; `Response::SystemFonts` mémorisée dans `edit.system_fonts`.
- Les 3 familles standard gardent `TextStyle.family/bold/italic` ; une police système met `TextStyle.font_file = Some(path)`.

### 5.7 Pages : recadrer, redimensionner, insérer (`pagesetup.rs`)

- État : `pagesetup: PageSetupState { dialog: Option<PageDialog> }` avec `PageDialog::{ Crop(CropForm), Resize(ResizeForm), Blank(BlankForm) }` (champs en mm, convertis ×72/25,4 à l'envoi). Dialogues par `dialog::show` (ids `"crop"`, `"resize-pages"`, `"blank-page"`, largeur 460, icône `glyph::CROP`/`glyph::PAGES`/`glyph::ADD`), pied : action principale (`dialog::primary`) + `dialog::secondary` (`dialog.password.cancel`).
- *Recadrer* : 4 `DragValue` de marges (0..demi-page), étendue résultante `pagesetup.crop.result`, avertissement `pagesetup.crop.too_small` (action principale non dessinée si < 18 pt), portée page courante / toutes, bouton secondaire « Rétablir le cadre d'origine » (`ResetCrop`). **Aperçu en direct** : `paint_crop_preview(painter, page_rect, size)` appelée dans `document_area` quand le dialogue est ouvert : quatre rectangles noirs à 43 % d'alpha hors zone conservée + liseré (aucune dépendance à la rotation : espace d'affichage).
- *Redimensionner* : préréglages `A3/A4/A5/Letter/Legal` (littéraux non traduits, exception de `I18N.md`), « Personnalisé », paysage, modes `KeepContent` / `ScaleContent` (note permanente `pagesetup.resize.scale_note`), portée.
- *Page vierge* : taille (comme la page courante / préréglage / personnalisée), nombre (1..=500), position avant/après la page courante/à la fin → `at`.
- Succès : `status.pages_cropped|resized|blank_inserted` posé au retour de `DocumentChanged` via `edit.pending_status: Option<String>` (le worker ne renvoie pas de compteur ; le compte vient de l'UI). La garde `handle_input` (`app.rs:591-596`) ajoute `|| self.pagesetup.dialog.is_some()`.
- Organiser (F8) : bouton « Page vierge » dans `organize_toolbar` (optionnel, ouvre le même dialogue avec la sélection comme position).

### 5.8 Clavier et presse-papiers (corrige D2)

Dans `handle_input`, **après** les gardes de champ de saisie (`app.rs:599-626`, elles `return` si un dialogue/une saisie a le focus) et avant la boucle `key(...)` :
```rust
let (copy, cut, paste) = ctx.input(|i| (
    i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
    i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
    i.events.iter().find_map(|e| if let egui::Event::Paste(s) = e { Some(s.clone()) } else { None })));
let objects = self.annotate.tool == Tool::SelectObject && self.edit.selection.is_some();
if copy { self.execute(ctx, if objects { Command::CopyObjects } else { Command::CopyText }); }
if cut && objects { self.execute(ctx, Command::CutObjects); }
if let (Some(s), Some(clip)) = (paste, &self.edit.clip) { if s.trim() == clip.marker.trim() && self.annotate.tool == Tool::SelectObject { self.execute(ctx, Command::PasteObjects); } }
```
- `key(COMMAND, Key::C, CopyText)` (ligne 738) est supprimé (mort). Ajouts : `key(COMMAND, Key::A, SelectAllObjects)` (consommé seulement avec l'outil Sélection d'objet), `key(NONE, F5, ToggleObjects)`.
- **Marqueur** : `Event::Paste` n'existe que si le presse-papiers système contient du texte (`EW/lib.rs:1027-1033`). À la copie d'objets, `ctx.copy_text(marker)` avec `marker = texte des objets texte` (utile aussi ailleurs) ou `[SuniPDF:<n> objet(s)]` ; `edit.clip = Some(ClipInfo { count, marker, source_doc: chemin, source_page, pastes: 0 })`. Coller seulement si le texte reçu **égale** le marqueur : si l'utilisateur a copié autre chose entre-temps, rien n'est collé. Le menu/la palette/la barre d'objet appellent `PasteObjects` directement (sans passer par le presse-papiers système).
- Collage : mêmes document et page que la source → décalage `(12·(pastes+1), −12·(pastes+1))` puis `pastes += 1` ; sinon `(0,0)`. Page cible = page courante ; `reselect_many = (page, Pick::Last(1))`.
- `execute` : `CopyObjects` (envoie `Request::CopyObjects`; la réponse `Copied` fait `copy_text` + `status.objects_copied`) ; `CutObjects` (idem puis `DeleteObjects`, `status.objects_cut`) ; `PasteObjects` (si `edit.clip.is_none()` → `status.paste_empty`) ; `SelectAllObjects` ; `ReplaceImage` (`rfd::FileDialog` avec les filtres d'`AddImage`, `editor.rs:390-396`, puis `EditOp::ReplaceImage`) ; `ToggleObjects` ; `CropPages`/`ResizePages`/`InsertBlankPage` (ouvrent les dialogues).

### 5.9 Onglets, annulation, état global

- `take_tab`/`put_tab` (`tabs.rs:52-100`) appellent `self.edit.reset_for_tab()` : efface `selection`, `reselect*`, `drag_mode`, `text_prompt`, `last_prompt`, `objects`, `objects_page`, `hover_index` ; **ne touche pas** à `clip` (global, cross-onglets), `system_fonts`, couleurs/épaisseur/opacité par défaut, `objects_open` (comme `bookmarks_open`/`comments_open`).
- Annuler/Rétablir (`Request::Undo/Redo`) → `DocumentChanged` → resélection nulle (`edit.selection = None`, comportement actuel) ; le panneau Objets se rafraîchit.
- Tout passe par `State::edit` (un geste = un instantané) ; copier ne crée pas d'entrée d'annulation et ne marque pas le document modifié.

### 5.10 Commandes à ajouter (`CORE/command.rs`, `REGISTRY`, ordre d'affichage des menus)

| Variante | id | clé libellé | clé raccourci | Menu | Onglet d'outils | `needs_document` |
|---|---|---|---|---|---|---|
| `ToggleObjects` | `view.objects` | `menu.view.objects` | `shortcut.objects` (F5) | View | Éditer, Tous | oui |
| `CopyObjects` | `edit.copy_objects` | `menu.edit.copy_objects` | `shortcut.copy_objects` | Edit | Éditer | oui |
| `CutObjects` | `edit.cut_objects` | `menu.edit.cut_objects` | `shortcut.cut_objects` | Edit | Éditer | oui |
| `PasteObjects` | `edit.paste_objects` | `menu.edit.paste_objects` | `shortcut.paste_objects` | Edit | Éditer | oui |
| `SelectAllObjects` | `edit.select_all_objects` | `menu.edit.select_all_objects` | `shortcut.select_all_objects` | Edit | Éditer | oui |
| `ReplaceImage` | `edit.replace_image` | `menu.edit.replace_image` | — | Edit | Éditer | oui |
| `CropPages` | `pages.crop` | `menu.pages.crop` | — | Pages | Pages, Tous | oui |
| `ResizePages` | `pages.resize` | `menu.pages.resize` | — | Pages | Pages | oui |
| `InsertBlankPage` | `pages.insert_blank` | `menu.pages.insert_blank` | — | Pages | Pages, Tous | oui |

À chaque variante : entrée `cmd!` dans `REGISTRY` (**jamais dans `Menu::File`** : le test `every_command_variant_is_registered` compte 7 commandes `File`, `command.rs:648`), bras exhaustif dans `shell::command_icon` (`shell.rs:246-309`, pas de bras par défaut) et dans `SuniApp::execute` (`app.rs:793-1009`), `command_color` (`CAT_EDIT` pour les commandes d'objets, `Menu::Pages` donne `CAT_PAGES`), liste de `ToolTab::commands` (ou, si `tool-hub.md` est livré avant, les groupes de `workspace` — `tool-hub.md:30,118,181`), activation dans `tools_panel` (`shell.rs:601-607`) : Copier/Couper/Remplacer = sélection non vide ; Coller = `edit.clip.is_some()`. Icônes : `ToggleObjects` `LAYERS`, `CopyObjects` `DUPLICATE`, `CutObjects` `CUT`, `PasteObjects` `PASTE`, `SelectAllObjects` `SELECT_ALL`, `ReplaceImage` `PICTURE`, `CropPages` `CROP`, `ResizePages` `FIT_PAGE`, `InsertBlankPage` `ADD`.

### 5.11 Pages tournées côté UI

Aucune modification de `annotate::to_pdf`, `paint_selection`, `paint_matches`, `paint_object_selection` : `page_h_pt` est déjà la hauteur **affichée** et les coordonnées reçues du moteur sont maintenant des coordonnées d'affichage (§3.0). À ajouter seulement : un test unitaire de `to_pdf` (aller-retour avec la conversion inverse) pour verrouiller la convention.

---

## 6. CLI (`apps/sunipdf-cli`) — anglais, JSON, codes d'erreur stables (D009)

| Sous-commande | Arguments | Sortie JSON |
|---|---|---|
| `objects FILE --page N [--password P]` | liste les objets d'une page | `{"page":N,"rotation":90,"size":{"width_pt":…,"height_pt":…},"objects":[{"index":0,"kind":"text","bounds":[l,b,r,t],"angle":0.0,"opacity":1.0,"text":"…","font":"Helvetica","fill":"#rrggbb"}, …]}` (coordonnées d'affichage) |
| `fonts [--filter TEXT]` | polices système `.ttf` | `[{"name":"Arial Bold","path":"C:\\Windows\\Fonts\\arialbd.ttf","size_bytes":…}]` |
| `add-text FILE --page N --x X --y Y --text T [--size 12] [--color 000000] [--opacity 1] [--font-file PATH \| --font-name NAME] [--bold] [--italic] --out OUT` | texte (police système incorporée si demandée) | `{"out":…,"added":2,"embedded_font":"Arial"\|null}` ; erreurs `missing_glyphs`, `font_not_embeddable` |
| `set-opacity FILE --page N --object I[,J…] --opacity 0.5 --out OUT` | | `{"out":…,"objects":2,"opacity":0.5}` |
| `rotate-object FILE --page N --object I --degrees 30 --out OUT` | rotation libre | `{"out":…,"angle":30.0}` |
| `copy-objects FILE --from-page A --objects I,J --to-page B [--into FILE2] [--dx 0] [--dy 0] --out OUT` | copier-coller | `{"out":…,"copied":2,"pasted_index":17}` |
| `replace-image FILE --page N --object I --image IMG --out OUT` | | `{"out":…,"bounds":[l,b,r,t]}` |
| `crop FILE [--pages all\|1,3-5] (--left L --top T --right R --bottom B [--unit pt\|mm] \| --reset) --out OUT` | | `{"out":…,"cropped":3,"sizes":[{"index":1,"width_pt":…,"height_pt":…}]}` |
| `resize-pages FILE [--pages …] --width W --height H [--unit pt\|mm] [--mode keep\|scale] --out OUT` | | `{"out":…,"resized":3}` |
| `insert-blank FILE --at N [--count 1] (--width W --height H [--unit] \| --like PAGE) --out OUT` | `--at 0` = fin | `{"out":…,"inserted":1,"page_count":4}` |

Aussi : `info` ajoute `"rotation": 0\|90\|180\|270` à chaque page (`page_rotation`, cheap) ; `search` documente que `rects` est en espace d'affichage (changement de comportement **uniquement** sur pages tournées/rognées, signalé dans `CHANGELOG`). Réutiliser `parse_pages`, `parse_hex_color`, `write_atomic` via `save_as`. Test unitaire de la CLI : `parse_margins`/`parse_unit` (mm → pt).

---

## 7. i18n — clés à ajouter dans `assets/i18n/fr-FR.json` **et** `en-US.json` (mêmes `{paramètres}`)

Contrôles automatiques : `cargo test -p sunipdf-i18n -p sunipdf-desktop`, `scripts\check-i18n.ps1`. Étendre le test `ui_keys_used_by_app_exist` (`app.rs:1964`) pour inclure `editor.rs`, `objects.rs`, `inline_text.rs`, `pagesetup.rs` dans la concaténation `include_str!` (il vérifie que tout littéral « a.b » en minuscules est une clé existante : éviter dans ces fichiers tout autre littéral de cette forme ; les `Id::new("object-bar")` à tiret sont ignorés).

**Registre (libellés de menu/palette et raccourcis)**

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.view.objects` | Panneau des objets | Objects Panel |
| `shortcut.objects` | F5 | F5 |
| `menu.edit.copy_objects` | Copier les objets sélectionnés | Copy Selected Objects |
| `shortcut.copy_objects` | Ctrl+C | Ctrl+C |
| `menu.edit.cut_objects` | Couper les objets sélectionnés | Cut Selected Objects |
| `shortcut.cut_objects` | Ctrl+X | Ctrl+X |
| `menu.edit.paste_objects` | Coller les objets | Paste Objects |
| `shortcut.paste_objects` | Ctrl+V | Ctrl+V |
| `menu.edit.select_all_objects` | Sélectionner tous les objets | Select All Objects |
| `shortcut.select_all_objects` | Ctrl+A | Ctrl+A |
| `menu.edit.replace_image` | Remplacer l'image… | Replace Image… |
| `menu.pages.crop` | Recadrer les pages… | Crop Pages… |
| `menu.pages.resize` | Redimensionner les pages… | Resize Pages… |
| `menu.pages.insert_blank` | Insérer une page vierge… | Insert Blank Page… |

**État / barre d'état**

| Clé | fr-FR | en-US |
|---|---|---|
| `status.objects_selected` | {count} objets sélectionnés : glisser pour déplacer, flèches pour décaler, Suppr pour supprimer, Ctrl+C / Ctrl+X pour copier ou couper. | {count} objects selected: drag to move, arrows to nudge, Del to delete, Ctrl+C / Ctrl+X to copy or cut. |
| `status.objects_copied` | {count} objet(s) copié(s) | {count} object(s) copied |
| `status.objects_cut` | {count} objet(s) coupé(s) | {count} object(s) cut |
| `status.objects_pasted` | Objets collés : ils forment un groupe (déplaçable, mais son texte n'est plus modifiable). | Objects pasted: they form a group (movable, but its text is no longer editable). |
| `status.paste_empty` | Rien à coller : copiez d'abord des objets (outil Sélection d'objet). | Nothing to paste: copy objects first (Select Object tool). |
| `status.rotating` | Rotation : {angle}° (Maj : pas de 15°) | Rotation: {angle}° (Shift: 15° steps) |
| `status.image_replaced` | Image remplacée | Image replaced |
| `status.pages_cropped` | {count} page(s) recadrée(s) | {count} page(s) cropped |
| `status.pages_resized` | {count} page(s) redimensionnée(s) | {count} page(s) resized |
| `status.blank_inserted` | {count} page(s) vierge(s) insérée(s) | {count} blank page(s) inserted |

**Barre d'objet, types, accessibilité**

| Clé | fr-FR | en-US |
|---|---|---|
| `bar.opacity` | Opacité | Opacity |
| `bar.opacity_hint` | Opacité de l'objet sélectionné (0 à 100 %) ; sans sélection : valeur des prochaines formes et images | Opacity of the selected object (0–100%); with no selection: value for the next shapes and images |
| `bar.angle` | Angle | Angle |
| `bar.angle_hint` | Angle de rotation en degrés, sens horaire (Entrée pour appliquer) | Rotation angle in degrees, clockwise (Enter to apply) |
| `bar.replace_image` | Remplacer l'image | Replace image |
| `bar.group` | Groupe de {count} objets | Group of {count} objects |
| `object.kind.form` | Groupe | Group |
| `a11y.rotate_handle` | Poignée de rotation de l'objet sélectionné | Rotation handle of the selected object |
| `a11y.objects_list` | Liste des objets de la page | List of the page's objects |

**Panneau Objets**

| Clé | fr-FR | en-US |
|---|---|---|
| `objects.title` | Objets | Objects |
| `objects.page` | Page {page} | Page {page} |
| `objects.empty` | Aucun objet de contenu sur cette page. | No content objects on this page. |
| `objects.hint` | Clic : sélectionner · Ctrl+clic : ajouter ou retirer · Les objets du haut sont devant. | Click: select · Ctrl+click: add or remove · Objects at the top are in front. |
| `objects.forward` | Avancer d'un plan | Bring forward |
| `objects.backward` | Reculer d'un plan | Send backward |
| `objects.delete` | Supprimer l'objet | Delete object |

**Texte en place et polices**

| Clé | fr-FR | en-US |
|---|---|---|
| `text.inline.hint` | Entrée : valider · Maj+Entrée : nouvelle ligne · Échap : annuler | Enter: apply · Shift+Enter: new line · Esc: cancel |
| `text.font.search` | Rechercher une police… | Search fonts… |
| `text.font.standard` | Polices standard | Standard fonts |
| `text.font.system` | Polices du système | System fonts |
| `text.font.loading` | Chargement des polices… | Loading fonts… |
| `text.font.none` | Aucune police ne correspond. | No matching font. |
| `text.font.embed_note` | La police est incorporée en entier dans le PDF (≈ {size} Mo). | The whole font file is embedded in the PDF (≈ {size} MB). |
| `text.font.style_note` | Gras / italique : choisissez la variante dans la liste. | Bold / italic: pick the variant in the list. |
| `error.missing_glyphs` | La police choisie ne contient pas tous les caractères saisis. Choisissez une autre police. | The chosen font does not contain every typed character. Pick another font. |
| `error.font_not_embeddable` | Cette police ne peut pas être incorporée (licence, contours PostScript, collection ou taille). | This font cannot be embedded (licence, PostScript outlines, collection or size). |

**Dialogues de pages**

| Clé | fr-FR | en-US |
|---|---|---|
| `pagesetup.scope` | Pages concernées | Pages |
| `pagesetup.scope.current` | Page courante | Current page |
| `pagesetup.scope.all` | Toutes les pages | All pages |
| `pagesetup.crop.title` | Recadrer les pages | Crop pages |
| `pagesetup.crop.subtitle` | Masque les bords des pages. Le contenu hors cadre reste dans le fichier : ce n'est pas un caviardage. | Hides the page edges. Content outside the frame stays in the file: this is not redaction. |
| `pagesetup.crop.left` | Marge gauche (mm) | Left margin (mm) |
| `pagesetup.crop.top` | Marge haute (mm) | Top margin (mm) |
| `pagesetup.crop.right` | Marge droite (mm) | Right margin (mm) |
| `pagesetup.crop.bottom` | Marge basse (mm) | Bottom margin (mm) |
| `pagesetup.crop.result` | Page résultante : {width} × {height} mm | Resulting page: {width} × {height} mm |
| `pagesetup.crop.too_small` | La zone restante est trop petite. | The remaining area is too small. |
| `pagesetup.crop.reset` | Rétablir le cadre d'origine | Restore the original frame |
| `pagesetup.crop.ok` | Recadrer | Crop |
| `pagesetup.resize.title` | Redimensionner les pages | Resize pages |
| `pagesetup.resize.subtitle` | Change la taille des pages, avec ou sans mise à l'échelle du contenu. | Changes the page size, with or without scaling the content. |
| `pagesetup.resize.preset` | Format | Size |
| `pagesetup.resize.custom` | Personnalisé | Custom |
| `pagesetup.resize.width` | Largeur (mm) | Width (mm) |
| `pagesetup.resize.height` | Hauteur (mm) | Height (mm) |
| `pagesetup.resize.landscape` | Paysage | Landscape |
| `pagesetup.resize.mode` | Contenu | Content |
| `pagesetup.resize.mode_keep` | Conserver le contenu tel quel (ancré en haut à gauche) | Keep the content as is (anchored top-left) |
| `pagesetup.resize.mode_scale` | Adapter le contenu à la nouvelle taille (centré) | Fit the content to the new size (centred) |
| `pagesetup.resize.scale_note` | Les annotations et liens existants ne suivent pas la mise à l'échelle. | Existing annotations and links do not follow the scaling. |
| `pagesetup.resize.ok` | Redimensionner | Resize |
| `pagesetup.blank.title` | Insérer une page vierge | Insert a blank page |
| `pagesetup.blank.subtitle` | Ajoute des pages blanches dans le document. | Adds blank pages to the document. |
| `pagesetup.blank.size` | Taille | Size |
| `pagesetup.blank.same_size` | Comme la page courante | Same as the current page |
| `pagesetup.blank.count` | Nombre de pages | Number of pages |
| `pagesetup.blank.position` | Position | Position |
| `pagesetup.blank.before` | Avant la page courante | Before the current page |
| `pagesetup.blank.after` | Après la page courante | After the current page |
| `pagesetup.blank.end` | À la fin du document | At the end of the document |
| `pagesetup.blank.ok` | Insérer | Insert |

Total : **≈ 75 clés**. Réutilisées sans changement : `dialog.password.cancel`, `object.kind.*`, `dialog.text.*`, `font.*`, `tools.images.*`, `bar.stroke|width|fill|apply*`. Libellés non traduits (symboles, exception de `docs/I18N.md`) : `A3`, `A4`, `A5`, `Letter`, `Legal`, suffixes ` mm`, ` %`, `°`.

---

## 8. Tests

### 8.1 Fixture à générer

`scripts/make_fixtures.py` (Python sans dépendance, même style que `two_pages`) : ajouter `boxes(path)` et l'appeler depuis `__main__` pour produire `crates/sunipdf-core/tests/fixtures/boxes.pdf` (commande documentée dans `CLAUDE.md` : `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`). Quatre pages Letter, police Helvetica ; chaque page contient un carré **rouge** `1 0 0 rg`, un carré **bleu** `0 0 1 rg` et le texte « BOXES x » à des positions *utilisateur* connues :

| Page | Boîtes / rotation | Rouge (utilisateur) | Bleu (utilisateur) | Texte (origine) |
|---|---|---|---|---|
| A | `/MediaBox [0 0 612 792] /Rotate 90` | `50 50 100 100 re` | `460 640 100 100 re` | `(BOXES A)` en `200 400` |
| B | idem `/Rotate 180` | idem | idem | idem |
| C | `/MediaBox [100 50 712 842] /CropBox [150 100 650 700] /Rotate 270` | `160 110 100 100 re` | `540 590 100 100 re` | `(BOXES C)` en `300 400` |
| D | `/MediaBox [0 0 612 792] /CropBox [100 100 500 600]` (sans rotation) | `110 110 100 100 re` | `390 490 100 100 re` | `(BOXES D)` en `200 300` |

```python
def boxes(path):
    pages = [  # (attributs de page, flux de contenu)
        (b"/MediaBox [0 0 612 792] /Rotate 90",  c(50, 50, 460, 640, 200, 400, "A")),
        (b"/MediaBox [0 0 612 792] /Rotate 180", c(50, 50, 460, 640, 200, 400, "B")),
        (b"/MediaBox [100 50 712 842] /CropBox [150 100 650 700] /Rotate 270", c(160, 110, 540, 590, 300, 400, "C")),
        (b"/MediaBox [0 0 612 792] /CropBox [100 100 500 600]", c(110, 110, 390, 490, 200, 300, "D")),
    ]
    # objets : 1 Catalog, 2 Pages, 3 Font, puis pour chaque page (Page, Contenu) ; Kids = les pages ; Resources /Font << /F1 3 0 R >>
```
(`c(rx, ry, bx, by, tx, ty, nom)` = `1 0 0 rg rx ry 100 100 re f 0 0 1 rg bx by 100 100 re f 0 0 0 rg BT /F1 24 Tf tx ty Td (BOXES nom) Tj ET`.)
Les autres tests fabriquent leurs images PNG à la volée (`image::save_buffer`, comme `edit_page_content_text_image_shapes`) et leurs copies de travail dans `std::env::temp_dir()`.

### 8.2 Tests d'intégration à ajouter dans `crates/sunipdf-core/tests/engine.rs`

Chacun utilise `engine()` (moteur partagé et sérialisé), vérifie par **relecture** (`page_objects`, `page_text`, `save_as` puis `open`) **et** par **pixels rendus** (`render_page(p, 1.0)`, lecture `rgba[(y*w+x)*4..]`, y compté depuis le haut = `hauteur_affichée − Y`). Tolérances : 1,5 px pour les rectangles, 4 px pour les boîtes de texte.

| Test | Assertions |
|---|---|
| `geometry_matches_rendered_pixels_on_rotated_and_cropped_pages` | Pour chaque page de `boxes.pdf` : `page_size` = taille attendue (A : 792×612, B : 612×792, C : cadre 500×600 → après rotation 270 : **600×500**, D : 400×500) ; boîte englobante des pixels rouges/bleus du rendu = `rect_to_display` du rectangle utilisateur (±1,5 px) ; `object_info` du tracé rouge (`fill_rgb == (255,0,0)`) a `bounds` = même rectangle ; `object_at(p, centre_rouge)` renvoie ce tracé et `object_at` hors de tous les objets renvoie `None`. |
| `text_and_markup_rects_follow_display_space_on_rotated_pages` (H2) | Page A et C : `search_page(p,"BOXES")[0].rects[0]` ≈ boîte des pixels sombres du texte ; `text_in_rect(p, rect_attendu).text == "BOXES A"` ; `add_markup(Highlight, [rect])` puis `annotations(p)` : `rect` ≈ rect d'affichage et pixels jaunes dans la zone ; `add_note(p, x, y, "n")` : `rect.left ≈ x`. |
| `edits_land_where_displayed_on_rotated_pages` | Sur A, B, C : `add_shape(Rectangle, magenta, rect d'affichage)` → pixel magenta au centre ; `add_text` « Droit » → `angle_deg ≈ 0` (±0,5), `bounds.left ≈ x`, `object_at` sur son centre, pixels sombres autour ; `add_image` 40×20 vert (largeur 120) → pixel vert au centre, `bounds` ≈ attendu ; `move_object(dx=40, dy=0)` → `bounds.left += 40` et `bottom` inchangé ; `resize_object` → `bounds` demandés ±1 ; `rotate_object(90)` → largeur/hauteur permutées ; sauvegarde + réouverture : mêmes pixels. |
| `free_rotation_reports_angle_and_oriented_quad` | Rectangle 100×50 pivoté de 30° : `angle_deg ≈ 30` (±0,5), `bounds` ≈ 111,6 × 93,3 (±2), centre inchangé ; pivoté encore de 15° → 45° ; `rotate_object(NaN)` et `±inf` → `Err` ; texte pivoté de 20° : arête la plus longue de `quad` à 20° ±1 ; `object_at` au coin de la boîte englobante hors du quad → `None`, au centre → `Some` ; pixel blanc hors du rectangle pivoté, couleur dedans. |
| `opacity_text_shapes_images_and_style_preservation` (H6, D6) | Rectangle magenta 50 % sur blanc → canal G ≈ 127 (±4), R=B=255 ; `set_object_style(fill_rgb: vert)` **sans** `opacity` → alpha conservé (G ≈ 207) ; `opacity: Some(1.0)` → vert plein ; image verte 40×20 ajoutée par `add_image_with(…, 0.5)` → pixel (127,207,127) ±4 puis `set_objects_style(opacity 1.0)` → (0,160,0) ; trait seul (ligne) à 50 % plus clair que plein ; texte 50 % : `ObjectInfo.opacity ≈ 0,5` ; persistance après `save_as`/`open` ; `opacity` bornée (`2.0` → 1, `-1` → 0). |
| `system_font_embeds_unicode_and_rejects_missing_glyphs` (H3, D7) | Ignoré avec message si `system_fonts()` est vide. Police « Arial » (sinon « Segoe UI », sinon la première) : `add_text("Привет Ωmega ł é", font_file)` → `Ok(1)` ; `page_text` contient « Привет » ; après `save_as` + réouverture le texte est toujours extrait et le rendu a des pixels sombres ; taille du fichier : +≥ moitié de la police avec la 1re insertion, +< 20 Ko avec la 2e insertion de la **même** police (cache) ; `add_text("日本", Arial)` → `Err(MissingGlyphs)` et `page_objects` inchangé ; fichier non-police → `Err(FontNotEmbeddable)` ; `add_text` multi-ligne `"a\nb"` → 2 objets, bas de `b` ≈ bas de `a` − 1,2×taille. |
| `multi_object_ops_keep_order_and_are_atomic` | 3 rectangles superposés : `objects_in_rect` `Contained` vs `Touching` (comptes attendus) ; `move_objects([0,2], 30, -20)` : bornes des deux déplacées, le 3e inchangé ; `reorder_objects` Front/Back/Forward/Backward vérifiés **au pixel** du recouvrement et par les index renvoyés ; `delete_objects([2,0,0])` (désordre + doublon) → reste l'objet 1 ; index invalide → `Err` et nombre d'objets inchangé. |
| `object_clipboard_copy_cut_paste_across_pages_documents_and_rotation` (H5) | Copie de 2 objets (forme + texte) de la page 0 : le document est **inchangé** (liste d'objets identique, ordre Z conservé : D5) ; `paste_objects(1, clip, 0, 0)` : +1 objet de type `Form`, bornes ≈ `clip.bounds` ±1,5, pixel de la forme au bon endroit ; décalage `(50,-30)` ; collage dans un **second** `Document` ouvert par `engine.open_bytes(doc.snapshot())` ; collage sur la page A de `boxes.pdf` (R90) d'un objet venant d'une page non tournée : `angle_deg ≈ 0` (compensation) et pixel visible ; coupe = `copy_objects` + `delete_objects` ; indices vides/invalides → `Err` ; après `save_as`/`open` le groupe collé est conservé. |
| `replace_image_keeps_frame_and_updates_pixels` | PNG vert 40×20 ajouté (largeur 200) puis remplacé par un PNG rouge 20×40 : toujours une `Image`, nouveau cadre = « contenir » centré dans l'ancien (±1), pixel rouge au centre, plus de vert aux coins de l'ancien cadre ; image pivotée de 90° : cadre inchangé (étiré) ; objet non image → `Err` ; fichier manquant → `Err` et document intact ; persistance après réouverture. |
| `crop_resize_and_blank_pages` (H4, H7) | `crop_page(0, 36 pt)` : taille 540×720, rendu de la même taille, pixel du rectangle bleu décalé de 36 pt ; objets toujours présents (le recadrage ne supprime rien) ; `reset_crop` → 612×792 ; recadrage sur la page A (R90, marge gauche 10) : bords du rendu et pixel rouge conformes ; marges négatives / zone < 18 pt → `Err` ; taille conservée après `save_as`/`open` ; `resize_page(KeepContent, 400×500)` : ancrage haut-gauche vérifié au pixel ; `resize_page(ScaleContent, 306×396)` : centre du rectangle bleu à la moitié des coordonnées (±2 px) — **si H4 échoue, ce cas devient `#[ignore]` documenté et le mode « adapter » est retiré de l'UI** ; `insert_blank_pages(1, 2, 300, 400)` : +2 pages, tailles, rendu entièrement blanc, rotation 0 ; `at > page_count`, taille ≤ 0 ou > 14 400 → `Err`. |
| `page_objects_listing_is_linear` (D4, garde de performance) | 200 objets texte ajoutés ; `page_objects` + `object_at` en moins de 5 s (borne large, pas un banc de mesure) ; 400 objets : durée < 4× celle de 200 (non quadratique). |

Régression : `edit_page_content_text_image_shapes`, `edit_resize_style_text_size_and_fonts`, `edit_rotate_duplicate_reorder`, `stamp_watermark_and_page_numbers`, `text_selection_and_annotations_round_trip` doivent rester verts **sans modification d'assertions** (corriger seulement les 3 littéraux `ShapeStyle`, §3.4) : preuve que l'identité tient en rotation 0.

### 8.3 Tests unitaires (sans PDFium, exécutables partout)

- `geometry.rs` : aller-retour `to_user(to_display(p))` pour les 4 rotations et une boîte décalée ; coins du §3.1 ; `upright_matrix` ; `vec_to_user`/`vec_to_display` inverses.
- `fonts.rs` : `parse_reg_output` sur un échantillon réel (CRLF, `Arial (TrueType)    REG_SZ    arial.ttf`, une ligne `.ttc`, une `(OpenType)`, une ligne vide, une ligne sans `REG_SZ`) ; `check_embeddable` sur des octets synthétiques (`OTTO` → Err, `ttcf` → Err, `fsType=0x0002` → Err, `0x0004` seul → Err, `0x0008` → Ok, `0` → Ok, table `OS/2` absente → Ok, trop court → Err).
- `edit.rs`/`clipboard.rs` : `reorder_plan` par simulation sur un `Vec<usize>` (Front/Back/Forward/Backward, sélection au bord, sélection contiguë).
- `boxes.rs` : `resize_matrix` (identité quand l'ancienne et la nouvelle page sont identiques ; échelle 0,5 ; page tournée).
- `command.rs` : tests existants (ids uniques, round-trip) ; le compte `File == 7` reste vrai.
- Application : `pointer_angle`, `snap_15`, `rotate_point`, `Selection::union_bounds`, fusion `Replace/Add/Toggle`, libellé tronqué sans casser un caractère Unicode, `mm_to_pt`, `crop_result_size`/validation, `to_pdf` (aller-retour avec l'inverse), tableau d'erreurs du test de clés étendu à `MissingGlyphs` et `FontNotEmbeddable`.

### 8.4 Scénarios de capture manuelle (`scripts\dev-screenshot.ps1 -Out x.png [-Pdf f] [-Commands 'id1,id2'] [-Drag 'x1,y1,x2,y2;…'] [-KeysAfter '^c^v']`)

Les coordonnées de `-Drag` sont des points logiques relatifs au coin haut-gauche de la fenêtre : **calibrer avec une première capture** (la mise en page dépend du zoom « largeur »). L'agent doit regarder chaque PNG produit (Read sur l'image) et consigner le résultat réel ; ne jamais annoncer « vérifié » sans l'avoir ouvert.

| # | Commande | Attendu |
|---|---|---|
| 1 | `-Pdf tests\fixtures\two-pages.pdf -Commands "tool.select_object,view.objects"` | panneau Objets à droite : texte « SuniPDF page 1 » et rectangle bleu, le dernier dessiné en haut |
| 2 | idem + `-Drag "<haut-gauche du rectangle>,<bas-droite>"` | rectangle élastique, objet sélectionné, ligne d'état |
| 3 | `-Drag` de clic sur le rectangle puis `-Drag` depuis la poignée ronde vers la droite | aperçu du cadre pivoté ; relâcher → rectangle pivoté, angle dans la barre d'objet |
| 4 | `-Commands "tool.select_object"` + élastique sur 2 objets puis `-Drag` pour les déplacer | déplacement groupé, une seule entrée Annuler |
| 5 | `-KeysAfter "^a^c^v"` (outil Sélection) | groupe collé décalé de 12 pt, barre d'état `status.objects_pasted` ; vérifie D2 (Ctrl+C/V réels) |
| 6 | `-Pdf tests\fixtures\boxes.pdf -Commands "tool.add_text"` + clic | texte droit à l'écran sur la page tournée A, éditeur en place visible avant validation |
| 7 | `-Commands "tool.select_object"` sur boxes.pdf page A + clic sur le carré rouge | cadre de sélection sur le carré, pas ailleurs (D1) |
| 8 | `-Commands "pages.crop"` | dialogue Recadrer + aperçu assombri sur la page |
| 9 | `-Commands "pages.resize"` / `"pages.insert_blank"` | dialogues cohérents avec `dialog::show` (en-tête, pied) |
| 10 | thème sombre + FR/EN pour 3, 8 | lisibilité, aucune clé manquante affichée |

---

## 9. Nouvelles dépendances : **aucune**

Rien n'est ajouté à `Cargo.toml` ni à `Cargo.lock` : `std::process::Command`/`std::fs` (liste des polices, lecture sfnt), `image` (déjà présent), `lopdf` (déjà présent, fusion de flux), `pdfium-render` 0.9.4 (déjà présent), `egui`/`eframe`/`rfd` (déjà présents).
Candidats étudiés et **non retenus** : `skrifa 0.44.0` / `read-fonts 0.41.0` (MIT OR Apache-2.0, déjà dans `Cargo.lock` via `epaint 0.36.2`) — inutiles tant que la vérification de couverture se fait par PDFium (repli si H3 échoue : l'ajouter à `sunipdf-core` pour `Charmap::map`, `skrifa-0.44.0/src/charmap.rs:102` ; aucune nouvelle crate téléchargée) ; `windows 0.62.2` (déjà dans l'arbre ; la feature `Win32_System_Registry` recompilerait une grosse crate) ; `subsetter` (MIT/Apache-2.0, sous-ensemble de polices : feuille de route, nécessite un ADR et du post-traitement lopdf).

---

## 10. Risques, pièges connus, ordre d'implémentation, estimation

### 10.1 Étape 0 — tests-sondes à écrire AVANT toute interface (≈ 1 jour)

Écrire un fichier **temporaire** `crates/sunipdf-core/tests/probes_edit_advanced.rs` (ou des tests `#[ignore]` dans `engine.rs`), exécuter, consigner le résultat réel dans `docs/PROJECT_STATUS.md`, puis transformer les sondes réussies en vrais tests (§8.2) et supprimer le reste.

| Hyp. | Ce qu'on suppose | Sonde | Si faux (repli) |
|---|---|---|---|
| H1 | `FPDFPageObj_GetBounds`/`GetRotatedBounds` sont en espace utilisateur (non tourné, origine de la MediaBox) | rotation 90° de la page 0 de `two-pages.pdf` ; `object_info(0,i).bounds` du carré bleu inchangé alors que le rendu l'a déplacé | aucun : c'est le contrat PDFium (objets de page), la sonde sert de garde |
| H2 | rectangles de texte (`search_page`, `text_in_rect`, `loose_bounds`) en espace utilisateur | même page : comparer `search_page("SuniPDF")` brut avec la boîte des pixels sombres du rendu | s'ils sont déjà en espace d'affichage, **ne pas convertir** les sorties texte (garder l'entrée `text_in_rect` en identité) ; Chromium les convertit avec `FPDF_PageToDevice`, ce qui plaide pour l'espace utilisateur |
| H3 | `load_true_type_from_bytes(…, true)` + `FPDFText_SetText` : texte Unicode correct, caractère absent → lecture inverse différente | Arial : « Привет Ωmega ł é » relu intégralement ; « 日本 » relu ≠ | si la relecture est trompeuse : pré-contrôle par `skrifa::Charmap::map` (déjà dans `Cargo.lock`, §9) ; si le texte CID ne s'affiche pas : renoncer aux polices système (limiter aux 14 standard) et le dire |
| H4 | `FPDFPage_TransFormWithClip` après changement des boîtes met le contenu à l'échelle et le rogne à la nouvelle boîte | carré bleu de `two-pages.pdf` : pixels après `resize_page(Scale)` | retirer le mode « adapter » (UI et CLI), garder « conserver » |
| H5 | `FPDF_NewXObjectFromPage` inter-documents fonctionne via `copy_into_x_object_form_object` ; bornes d'un `Form` serrées | copier 2 objets, coller dans un 2e `Document` ; comparer les bornes | repli : collage limité au même document par `copy_to_page` (mêmes appels que `duplicate_object`), presse-papiers cross-onglet retiré et documenté |
| H6 | l'alpha d'un objet image est écrit en ExtGState et rendu | image verte 50 % : pixel (127,207,127) | alpha incorporé dans le bitmap pour les **nouvelles** images (multiplication du canal alpha avant `set_image`), opacité des images existantes désactivée dans l'UI |
| H7 | `refresh_page_sizes` voit `set_crop`/`set_media` sans rechargement | taille après `crop_page` | `restore(snapshot())` après chaque changement de boîte |
| H8 | `\n` dans un texte unique : comportement actuel | affichage/relecture d'un texte « a\nb » avec le moteur actuel | information seulement (le plan découpe par ligne de toute façon) |

### 10.2 Ordre d'implémentation (chaque étape compile et passe ses tests ; ne pas empiler)

Commandes de contrôle à chaque étape (poste à 14 Go : **toujours `-j 2`**) : `cargo test -p sunipdf-core -j 2` ; `cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2` ; `cargo clippy --workspace --all-targets -j 2 -- -D warnings` ; `cargo fmt --all -- --check` ; `.\scripts\check-i18n.ps1`. En fin de tranche : `cargo test --workspace -j 2`. Rapporter les résultats **réels**.

| # | Étape | Fichiers | Livrable vérifiable |
|---|---|---|---|
| 0 | Sondes H1–H8 | `tests/probes_edit_advanced.rs` | tableau de résultats réels |
| 1 | `geometry.rs`, `Rotation` `pub(crate)`, fixture `boxes.pdf`, `make_fixtures.py` | core | tests unitaires + test « formules = pixels du rendu » |
| 2 | API d'affichage : `describe` (quad, angle, opacité, `Form`), une seule page texte, conversions du §3.3, `refresh_page_sizes` `pub(crate)` | `edit.rs`, `annot.rs`, `text.rs` | tests `geometry_matches…`, `text_and_markup…`, `edits_land_where_displayed…`, `page_objects_listing_is_linear` + tous les tests existants verts |
| 3 | Opacité : `ObjectStyle.opacity`, `ShapeStyle.opacity`, `add_image_with`, correctif D6 ; UI : champ d'opacité (objet unique) | `edit.rs`, `editor.rs`, `worker.rs` | test `opacity_…` ; capture |
| 4 | Rotation libre : poignée, `DragMode::Rotate`, champ Angle, cadre orienté | `editor.rs` | tests unitaires de géométrie UI ; test `free_rotation_…` ; capture 3 |
| 5 | Presse-papiers moteur : `copy_objects`, `paste_objects`, `duplicate_objects` (corrige D5) | `clipboard.rs`, `edit.rs` | test `object_clipboard_…` |
| 6 | Multi-objets : moteur (`move/delete/set_style/reorder/objects_in_rect`, `reorder_plan`) puis worker (`Pick`, `Selection`) puis UI (élastique, Maj+clic, Ctrl+A, groupe) ; **commandes** `SelectAllObjects`, `CopyObjects`, `CutObjects`, `PasteObjects` et correctif clavier D2 | `edit.rs`, `worker.rs`, `editor.rs`, `app.rs`, `shell.rs`, `command.rs`, `tabs.rs` (D3) | test `multi_object_ops_…` ; captures 2, 4, 5 |
| 7 | Panneau Objets | `objects.rs`, `app.rs`, `shell.rs` | capture 1 |
| 8 | Remplacer l'image | `edit.rs`, `editor.rs` | test `replace_image_…` |
| 9 | Pages : `boxes.rs` (+ CLI `crop`, `resize-pages`, `insert-blank`) puis dialogues | `boxes.rs`, `pagesetup.rs`, `app.rs`, `command.rs` | test `crop_resize_and_blank_pages` ; captures 8, 9 |
| 10 | Polices système : `fonts.rs`, cache, `add_text` révisé, `MissingGlyphs`/`FontNotEmbeddable` ; sélecteur de police dans le **dialogue** existant | `fonts.rs`, `edit.rs`, `error.rs`, `editor.rs`, `worker.rs` | tests unitaires + `system_font_…` ; CLI `fonts`, `add-text` |
| 11 | Édition en place (remplace le dialogue pour les cas éligibles) | `inline_text.rs`, `editor.rs`, `app.rs` | capture 6 ; essai manuel accents/Maj+Entrée/Échap |
| 12 | CLI restante (`objects`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`), i18n finale, documentation (§11), captures 1–10 (FR/EN, clair/sombre), clippy/fmt/tests complets | tout | `cargo test --workspace -j 2` vert, rapport réel |

Jalons : **J1** = étapes 0–4 (pages tournées corrigées, opacité, rotation libre) ; **J2** = 5–8 (sélection multiple, presse-papiers, panneau, remplacement d'image) ; **J3** = 9–12 (pages, polices, texte en place, CLI, docs). Chaque jalon peut être livré seul.

### 10.3 Risques et pièges

| # | Risque / piège | Parade |
|---|---|---|
| R1 | H2 faux ou partiellement faux (rectangles de texte) | étape 0 ; conversions isolées dans 4 fonctions (`text_in_rect`, `search_page`, `annotations`, `add_markup`) |
| R2 | `FPDFText_SetText` + police CID : comportement pour caractères absents/espaces | relecture `text()` (H3) ; test négatif « 日本 » ; message d'erreur typé ; repli `skrifa` |
| R3 | **Poids des PDF** : chaque police incorporée en entier ; les 50 niveaux d'annulation (`UNDO_MAX_BYTES` 512 Mo, `worker.rs:18-19`) contiennent chacun une copie du document | note de poids dans l'UI, cache par document, plafond 32 Mio par police, « Optimiser » retire les orphelins ; sous-ensemble en feuille de route |
| R4 | Objets `Form` (collés/dupliqués) : texte non éditable, couleur non modifiable, opacité non garantie | contrôles désactivés selon `ObjectKind` ; message de statut explicite |
| R5 | **Durées de vie PDFium** : `paste_objects` mélange `PdfDocument<'a>` du document et du PDF temporaire ; `copy_into_x_object_form_object(&self, &mut PdfDocument<'a>)` impose un même `'a` | écrire `impl<'a> Document<'a>` + `self.pdfium()` (`&'a Pdfium`, `lib.rs:191-195`) ; si l'emprunt échoue, passer par `PdfPageGroupObject::copy_into_x_object_form_object(destination)` (`group.rs:417-426`) ou par `Pdfium::bindings()` — jamais d'`unsafe` |
| R6 | `restore()` invalide les `PdfPage`/objets ouverts (cas `merge_content_streams`) | toujours réobtenir la page après (déjà le motif de `reorder_object`) ; vider `font_cache` |
| R7 | Régénération : les setters de couleur ne régénèrent pas | `page.regenerate_content()` en fin de chaque mutateur (§3.0) ; test par sauvegarde + réouverture dans chaque test |
| R8 | `Event::Paste` absent si le presse-papiers système n'a pas de texte (EW:1027-1033) | marqueur texte à la copie ; `PasteObjects` aussi accessible par menu/palette/barre |
| R9 | Police-proxy egui ≠ police PDF dans l'édition en place | documenté (§2) ; texte pivoté → dialogue ; aucune promesse WYSIWYG de largeur |
| R10 | `reg.exe` bloqué (stratégie de sécurité) ou page de code OEM | repli `read_dir` ; entrées sans fichier existant ignorées ; liste vide = combo « Aucune police » sans erreur |
| R11 | Glyphes Segoe MDL2 à confirmer (`CROP E7A8`, `PASTE E77F`, `CUT E8C6`, `SELECT_ALL E8B3`, `ADD E710`, `LAYERS E81E`) | repli texte déjà prévu ; vérifier sur capture (A1 de `KNOWN_ISSUES`) |
| R12 | Compensation de rotation au collage (`src_rot − dst_rot`) et dérive de pivot d'un tracé asymétrique | tests dédiés ; `ponytail:` dans le code avec plafond connu |
| R13 | Fusions de conflits : `command_icon`/`execute` sont exhaustifs, `tool-hub.md` supprime `ToolTab`, `sign-image.md` utilise `Request::ObjectInfo`/`edit.reselect` | additif uniquement dans `worker.rs` (§4) ; ajouter les commandes aux groupes de `tool-hub` s'il atterrit avant |
| R14 | Écrire dans un document avec signature numérique l'invalide (déjà vrai pour tous les outils) | hors tranche ; mention dans le guide |
| R15 | `FPDFPage_TransFormWithClip` ne bouge pas les annotations ; le recadrage ne supprime pas le contenu | notes permanentes dans les dialogues ; pas de bouton « Supprimer le contenu hors cadre » (caviardage = autre agent) |
| R16 | Coordonnées de `-Drag` des captures dépendantes de la fenêtre | calibrage par capture préalable ; ne pas déclarer « vérifié » sans lecture de l'image |

### 10.4 Coordination avec les autres plans de `docs/plans/`

- `continuous-view.md` : indépendant (le moteur parle en espace d'affichage ; `to_pdf` reste par page). Les appels `inline_text_editor`/`paint_crop_preview` se font par page-fente avec `page_rect` et `h_pt` de la page (cf. `continuous-view.md:542-546`).
- `tool-hub.md` : si livré **avant**, rattacher les 9 commandes aux groupes « Éditer » / « Organiser » du nouvel espace de travail et supprimer les références à `ToolTab` du §5.10 ; si livré **après**, il devra reprendre les nouvelles commandes (le compte `Menu::File == 7` reste vrai).
- `sign-image.md` : ses limites « pages pivotées » (`sign-image.md:79-82`) sont **levées par l'étape 2** ; il continue d'utiliser `Request::ObjectInfo`/`edit.reselect` (conservés, §4) et `TextStyle::default()` (nouveau champ `font_file: None`).
- `bookmarks-links.md` : `add_link{rect}` doit être en espace d'affichage (même convention, utiliser `geometry_of`).
- `compare-autosave.md` / `compress-lossy.md` : « Optimiser » est la parade aux polices/images orphelines ; aucun conflit de code attendu.

### 10.5 Estimation : **XL** (≈ 25 à 30 jours d'ingénieur)

J1 ≈ 8 j (géométrie 3, opacité 1,5, rotation 2, sondes 1, tests 0,5) ; J2 ≈ 10 j (multi-objets + `Selection` 4, presse-papiers 3, panneau 1,5, remplacer l'image 1,5) ; J3 ≈ 10 j (pages 3, polices 3, texte en place 3, CLI/docs 1). Les trois jalons sont indépendants côté valeur ; la partie la plus incertaine est H5 (presse-papiers inter-documents) puis H3.

---

## 11. Documentation à mettre à jour (au fur et à mesure, uniquement avec des résultats réels)

| Fichier | Mise à jour |
|---|---|
| `docs/PROJECT_STATUS.md` | cocher SUNI-031c **après** exécution (tests, captures) ; nouveaux totaux de tests ; retirer « polices système incorporées » de « Non réalisé » ; consigner les résultats des sondes H1–H8 |
| `docs/NEXT_TASKS.md` | retirer la section SUNI-031c ; ajouter SUNI-031d : sous-ensemble de polices, collage éditable, rotation/redimensionnement de groupe, recadrage à la souris, `.otf`/`.ttc` |
| `docs/FEATURE_MATRIX.md` | B : « Modification de texte existant » (rotation libre faite), « Ajout de texte » (polices système TrueType), lignes **Opacité**, **Panneau d'objets**, **Couper/copier/coller d'objets (groupe)** ; C : « pages vierges » **Fait**, lignes **Recadrer** (masquage) et **Redimensionner** |
| `docs/KNOWN_ISSUES.md` | A7 remplacé (rotation libre + polices système faites) ; ajouter : polices entières et TTF seul (D7), recadrage ≠ caviardage, mise à l'échelle sans annotations, collage = groupe, police-proxy de l'édition en place, orphelins → Optimiser, pages tournées corrigées (D1) + CLI `search` en coordonnées d'affichage, Ctrl+C/X/V corrigés (D2) ; numéros à attribuer à la livraison (d'autres plans en réservent) |
| `docs/DECISIONS.md` | ADR courts : **API moteur en espace d'affichage** ; **polices système : `FPDFText_LoadFont` CID/Identity-H, incorporation entière, énumération par registre, TTF seul, contrôle `fsType`** ; **presse-papiers d'objets = PDF d'une page + XObject formulaire + marqueur texte** ; **Couper = copier + supprimer** (numéros libres à la livraison) |
| `docs/ARCHITECTURE.md` | modules `geometry`, `fonts`, `clipboard`, `boxes` ; convention « coordonnées d'affichage » |
| `docs/ROADMAP.md` | éléments hors périmètre du §2 |
| `docs/I18N.md` | préfixes `pagesetup.*`, `objects.*`, `text.*`, `bar.*` étendus |
| `docs/USER_GUIDE.fr.md` / `.en.md` | éditer (rotation, polices, opacité, panneau Objets, sélection multiple, presse-papiers, texte en place) ; recadrer / redimensionner / page vierge ; avertissements (recadrage ≠ caviardage, police entière) |
| `CHANGELOG.md` | entrée : correction des pages tournées/rognées, changement de coordonnées de `sunipdf-cli search`, nouvelles commandes CLI |
| `CLAUDE.md` | section « Modules existants » (nouveaux modules) ; fixture `boxes.pdf` produite par la commande de régénération existante |
| `docs/THIRD_PARTY_NOTICES.md` | **aucun changement** (aucune dépendance ajoutée) |

### Questions ouvertes (à trancher par l'utilisateur ; valeurs par défaut du plan entre parenthèses)

1. Collage en v1 = **groupe non éditable** (oui) ; un collage natif éditable coûterait ≈ 6 j de plus.
2. Polices incorporées **en entier**, ≈ 1 Mo chacune (oui) ; sous-ensemble ≈ 6–8 j + ADR.
3. Mode « Adapter le contenu » du redimensionnement sans déplacer annotations/liens (oui, avec note ; supprimé si H4 échoue).
4. Ancrage du mode « Conserver le contenu » : haut-gauche (oui) ou centre.
5. Rectangle élastique : « entièrement contenu » par défaut, `Alt` = « touché » (oui).
6. Le correctif des raccourcis Ctrl+C/X/V (D2) écrit un marqueur texte dans le presse-papiers système à la copie d'objets (oui).
7. Ordre d'atterrissage relatif à `tool-hub` et `continuous-view` (indifférent pour le moteur ; l'UI se rebase).

