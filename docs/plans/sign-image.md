# Plan — tranche `sign-image` : signature manuscrite (image), paraphe, date

Rédigé le 2026-10-14, en lecture seule du dépôt (aucun `cargo` exécuté : rien de ce qui suit n'a été compilé).
Toutes les API citées ont été lues dans les sources locales. Abréviations des chemins :

- `PR/` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src/`
- `IMG/` = `…/image-0.25.10/src/`
- `EGUI/` = `…/egui-0.36.2/src/`, `EPAINT/` = `…/epaint-0.36.2/src/`
- `CHRONO/` = `…/chrono-0.4.45/`

Répartition conseillée : moteur → `pdf-editor` ; interface et catalogues → `ui-ux` (D013) ; relecture des
libellés d'avertissement et de la section sécurité → `security-signatures`.

## 1. Objectif et comportement visible

Correspond à la moitié « Sign » de l'outil PDFCraft **Fill & Sign**. Ce n'est **pas** l'outil « Use a
certificate » : aucune signature numérique, aucun certificat.

Ce que voit l'utilisateur :

1. Trois commandes nouvelles, dans le menu Édition, la palette Ctrl+K et les panneaux « Éditer » et
   « Tous les outils » : **Signer (signature manuscrite)…** (touche `S`), **Parapher (initiales)…** et
   **Insérer la date du jour**. Le remplissage « texte » est l'outil existant *Ajouter du texte* (`T`), déjà
   dans les mêmes panneaux : aucun code nouveau pour lui.
2. « Signer… » ouvre une boîte modale (cadre commun `dialog::show`) :
   - un sélecteur *Signature / Paraphe* ;
   - un **encadré permanent** : « Signature visuelle uniquement : elle ne prouve ni l'identité du signataire
     ni l'intégrité du document… Ce n'est pas une signature numérique certifiée. » ;
   - les signatures **déjà enregistrées sur ce poste** (vignettes sur fond blanc ; clic = placer ; corbeille
     en deux temps = supprimer) ;
   - *Dessiner* : un pavé blanc où l'on signe à la souris, au pavé tactile ou au stylet (encre noire ou
     bleue, épaisseur, « annuler le dernier trait », « effacer ») ;
   - *Importer une image* : PNG transparent utilisé tel quel, ou photo sur fond clair dont le fond est rendu
     transparent par un **seuil** réglable, avec aperçu immédiat ;
   - une case « Enregistrer sur cet ordinateur pour la réutiliser » (cochée par défaut), suivie du chemin de
     stockage et de la mention « sans chiffrement, jamais envoyée sur Internet ».
3. « Placer sur la page » ferme la boîte : le pointeur devient une croix et un **fantôme** de la signature
   suit la souris ; un clic la pose (centrée sous le clic, maintenue dans la page). Maj+clic la pose en
   gardant le mode actif (parapher plusieurs endroits) ; Échap annule.
4. La signature posée est un **objet image de la page** (comme *Insérer une image*), aussitôt sélectionné :
   poignées de redimensionnement (proportions conservées), glisser pour déplacer, flèches, rotation ±90°,
   dupliquer, supprimer, Ctrl+Z. Le fond transparent laisse voir le contenu situé dessous.
5. « Insérer la date du jour » : même mode de placement, pose un objet texte (`14/10/2026` en français,
   `10/14/2026` en anglais), modifiable ensuite par F2.

## 2. Faisabilité : **oui**

Tout se fait hors ligne avec ce qui est déjà dans l'arbre (PDFium, `image`, egui). Une seule dépendance
directe ajoutée : `chrono`, déjà présent dans `Cargo.lock`.

Point technique central, **lu dans les sources** : `Document::add_image` conserve déjà la transparence.

| Étape | Source |
|---|---|
| `image::open(path)` = `ImageReader::open(path)?.decode()` ; un PNG RGBA 8 bits donne `DynamicImage::ImageRgba8` | `crates/sunipdf-core/src/edit.rs:471` ; `IMG/images/dynimage.rs:1632-1637` |
| `PdfPageImageObject::new_with_size(&doc, &img, w, h)` → `new` → `set_image` | `edit.rs:478-483` ; `PR/pdf/document/page/object/image.rs:259-270` et `107-115` |
| `set_image` crée un bitmap **BGRA** (`PdfBitmap::empty(w, h, PdfBitmapFormat::BGRA)` → `FPDFBitmap_CreateEx`), y copie les octets RGBA permutés **avec l'octet alpha** (`as_rgba8()`, sinon `to_rgba8()`), puis `set_bitmap` → `FPDFImageObj_SetBitmap` | `image.rs:686-721` et `724-739` ; `PR/pdf/bitmap.rs:132-158` ; `PR/bindings.rs:3187-3203` et `8837-8849` |
| `FPDFBitmap_BGRA` : « Pixel components are independent of alpha » (non prémultiplié, comme `image::Rgba`) | `third_party/pdfium/win-x64/include/fpdfview.h:1115-1117` |
| PDFium, `CPDF_Image::SetImage` : si `IsAlphaFormat()`, le masque `CloneAlphaMask()` est écrit dans un flux DeviceGray 8 bits référencé par **`/SMask`** ; le flux RVB est écrit sans l'octet alpha | `core/fpdfapi/page/cpdf_image.cpp`, **amont PDFium, branche `main`, lu par requête web** (miroir GitHub `chromium/pdfium`) |

Réserve honnête sur la dernière ligne : le code C++ du build 7881 n'est pas sur ce poste (le tag
`chromium/7881` a répondu 404, seule la branche `main` a pu être lue, et à travers un résumé). **La preuve
est donc le test d'intégration de l'étape 1** (§8, `image_alpha_is_preserved_as_soft_mask`), premier jalon
avant toute interface. Plan B s'il échouait : ajouter le `/SMask` par lopdf après l'insertion, sur le motif
instantané → lopdf → `restore` de `merge_content_streams` (`edit.rs:402-421`).

Laissé de côté **volontairement** (à consigner dans `KNOWN_ISSUES.md`) :

- **Aucune valeur cryptographique.** Pas de certificat, pas de PAdES/CMS, pas d'horodatage, pas de
  vérification : tranche « Use a certificate » (crate `sunipdf-signatures`, P5).
- **Signature tapée** (nom rendu dans une police manuscrite) : dépend de l'incorporation de polices système
  (limite A7 de `KNOWN_ISSUES.md`).
- **Pression du stylet** : egui ne fournit que des positions ; trait d'épaisseur constante.
- **Chiffrement au repos des signatures enregistrées** : PNG en clair dans le profil utilisateur, protégé
  par les seules ACL de `%APPDATA%`. DPAPI possible plus tard, par ADR. Parade livrée : la case
  « Enregistrer… » peut être décochée, rien n'est alors écrit sur le disque.
- **Suppression du fond** : seuil de luminance global à rampe douce. Une photo à l'éclairage très inégal
  (ombre portée) demanderait un seuil adaptatif ; l'aperçu permet de juger avant de valider.
- **Pages pivotées (`/Rotate` ≠ 0)** : limite **existante** de tous les outils d'édition, non documentée à
  ce jour. `Document::page_size` renvoie la taille affichée (`crates/sunipdf-core/tests/engine.rs:273-277`)
  et `annotate::to_pdf` (`apps/sunipdf-desktop/src/annotate.rs:99-104`) ne compense pas la rotation : la
  signature y serait mal placée et pivotée. Correction transversale à planifier, hors tranche.
- **Parapher toutes les pages en une fois** : non livré (Maj+clic, page par page).
- **Document déjà signé numériquement** : toute modification, y compris une signature visuelle, invalide les
  signatures numériques existantes ; aucun avertissement aujourd'hui (question ouverte, §10).

## 3. API moteur (`sunipdf-core`)

### 3.1 `edit.rs` — insérer une image déjà décodée

```rust
impl Document<'_> {
    /// Insère une image déjà décodée de `width_pt` points de large, coin bas-gauche en (x, y).
    /// Le canal alpha est conservé (masque `/SMask` écrit par PDFium).
    pub fn add_image_data(
        &mut self,
        page_index: usize,
        image: &image::DynamicImage,
        x: f32,
        y: f32,
        width_pt: f32,
    ) -> Result<(), EngineError>;
}
```

- Corps : celui de l'actuel `add_image` à partir du contrôle de taille (`edit.rs:473-486`). `add_image`
  devient `image::open(path)` avec son message d'erreur inchangé (`edit.rs:471-472`), puis
  `self.add_image_data(page_index, &img, x, y, width_pt)`. Aucun changement de comportement pour l'existant.
- Validation en tête : `width_pt`, `x`, `y` finis, `width_pt > 0`, `image.width() > 0 && image.height() > 0`,
  sinon `EngineError::Other("invalid image size")`. Page hors limites : `PageOutOfRange` par `page_handle`
  (`pages.rs:282-289`).
- Appels : `PdfPageImageObject::new_with_size` (`PR/.../object/image.rs:259-270`), `translate` (macro
  `create_transform_setters!`, `image.rs:821-827`), `page.objects_mut().add_image_object(object)`
  (`PR/pdf/document/page/objects/common.rs:308-320`).
- **Régénération du contenu** : `add_image_object` la déclenche sous la stratégie par défaut
  `AutomaticOnEveryChange` (`PR/pdf/document/page.rs:150-153` et `196`) ; pas de `regenerate_content()`
  explicite, comme `add_image` aujourd'hui. Le nouvel objet est le **dernier** de `page_objects`.
- **Instantané** : rien de particulier. `State::edit` (`apps/sunipdf-desktop/src/worker.rs:372-393`) prend
  l'instantané avant `apply` et le restaure si l'opération échoue.
- Le type `image::DynamicImage` est celui que `pdfium-render` attend (une seule entrée `image 0.25.10` dans
  `Cargo.lock` ; `edit.rs:478` le prouve déjà).

### 3.2 Nouveau module `signature_image.rs` — fonctions pures, sans PDFium

`pub mod signature_image;` dans `crates/sunipdf-core/src/lib.rs` (liste des modules, lignes 10-19). Le nom
évite `sign`, réservé à la future signature numérique. En-tête du module : « Signature visuelle : préparation
d'une image. Aucune valeur cryptographique. »

```rust
/// Traits (points en pixels, origine en haut à gauche) → image RGBA : encre `rgb`, alpha = couverture.
pub fn rasterize_strokes(
    strokes: &[Vec<(f32, f32)>],
    width: u32,
    height: u32,
    pen_px: f32,
    rgb: (u8, u8, u8),
) -> image::RgbaImage;

/// Rend transparent le fond clair : alpha = min(alpha, rampe(seuil − luminance)).
pub fn remove_background(img: &image::RgbaImage, threshold: u8) -> image::RgbaImage;

/// Rogne à la boîte englobante des pixels d'alpha > 16, plus `padding` ; `None` si l'image est vide.
pub fn trim(img: &image::RgbaImage, padding: u32) -> Option<image::RgbaImage>;

/// Décode une image (limites de taille, orientation EXIF) et la réduit si un côté dépasse `max_side`.
pub fn load(path: &std::path::Path, max_side: u32) -> Result<image::RgbaImage, EngineError>;

/// Écrit un PNG RGBA de façon atomique.
pub fn save_png(img: &image::RgbaImage, path: &std::path::Path) -> Result<(), EngineError>;
```

Implémentation :

- **`rasterize_strokes`** (le crate `image` ne trace pas de lignes ; pas de dépendance pour 30 lignes).
  Tampon `alpha: Vec<u8>` de `width * height` ; rayon `r = max(pen_px / 2, 0.5)`. Pour chaque trait, les
  segments sont `windows(2)` ; un trait d'un seul point donne le segment dégénéré `[p, p]` (le point d'un
  « i »). Pour chaque segment, parcourir sa boîte englobante élargie de `r + 1`, bornée à l'image ; pour le
  centre de pixel `p = (x + 0.5, y + 0.5)` : `t = clamp(((p − a)·(b − a)) / |b − a|², 0, 1)` (0 si la
  longueur est nulle), `d = |p − (a + t(b − a))|`, `couverture = clamp(r + 0.5 − d, 0, 1)`,
  `alpha[i] = max(alpha[i], (couverture * 255) as u8)`. Sortie par `RgbaImage::from_fn`
  (`IMG/images/buffer.rs:1426`) : tous les pixels portent `rgb`, seul l'alpha varie (bords anti-crénelés,
  flux très compressible). Commentaire à laisser :
  `// ponytail: polyligne brute ; lissage (Chaikin) si les courbes rapides paraissent anguleuses`.
- **`remove_background`** : pour chaque pixel (`pixels_mut`, `buffer.rs:902`),
  `luma = (299 r + 587 g + 114 b) / 1000`, `a = clamp((threshold − luma) * 255 / 40, 0, 255)`,
  `p[3] = min(p[3], a)`. La rampe de 40 niveaux adoucit les bords ; les couleurs d'origine sont conservées
  (stylo bleu). Commentaire :
  `// ponytail: seuil global ; seuil adaptatif si les photos à ombre portée posent problème`.
- **`trim`** : balayage min/max des pixels d'alpha > 16 ; `None` si aucun ; marge bornée à l'image ;
  `image::imageops::crop_imm(img, x, y, w, h).to_image()` (`IMG/imageops/mod.rs:56-65`,
  `IMG/images/sub_image.rs:78`).
- **`load`** — frontière de confiance (fichier quelconque choisi par l'utilisateur) :
  1. `image::ImageReader::open(path)?.with_guessed_format()?` (`IMG/io/image_reader_type.rs:344` et `254`) ;
     les `io::Error` deviennent `EngineError::Io` par `#[from]` (`crates/sunipdf-core/src/error.rs:12`) ;
  2. `image::Limits::default()` (512 Mio d'allocation, `IMG/io/limits.rs:49-57`) complété par
     `max_image_width` / `max_image_height = Some(16_384)` — par affectation de champs, la structure étant
     `#[non_exhaustive]` — puis `reader.limits(limits)` (`image_reader_type.rs:137`) ;
  3. `reader.into_decoder()` (`image_reader_type.rs:219-224`, applique les limites) ; refuser si
     `decoder.total_bytes() > 512 Mio` (`IMG/io/decoder.rs:66`) : `decode()` fait cette réservation lui-même
     (`image_reader_type.rs:311-323`), pas `into_decoder` ;
  4. `decoder.orientation()` (`decoder.rs:53-58`, photos de téléphone), `DynamicImage::from_decoder`
     (`dynimage.rs:243`), `apply_orientation` (`dynimage.rs:1161`) ;
  5. `thumbnail(max_side, max_side)` **seulement si** `max(w, h) > max_side` : la fonction agrandirait une
     petite image (`dynimage.rs:913-917`) ; puis `into_rgba8()` (`dynimage.rs:418`).
  Erreurs de décodage : `EngineError::Other(format!("{}: {e}", path.display()))`, même forme que `edit.rs:472`.
- **`save_png`** : encodage en mémoire puis `write_atomic`, copie du motif de
  `crates/sunipdf-core/src/convert.rs:75-83` (`image::write_buffer_with_format(&mut Cursor::new(&mut png),
  img.as_raw(), w, h, image::ColorType::Rgba8, image::ImageFormat::Png)`, `dynimage.rs:1658-1668`).
  `write_atomic` crée le dossier au besoin (`pages.rs:63-79`).

Aucune nouvelle variante d'`EngineError`, aucun nouveau code d'erreur.

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

| Ajout | Où | Contenu |
|---|---|---|
| `EditOp::AddImageData { page: usize, image: Arc<image::DynamicImage>, x: f32, y: f32, width_pt: f32 }` | après `AddImage` (lignes 145-151) | image déjà décodée ; `Arc` car l'interface garde la même image pour la reposer |
| bras de `apply` | après le bras `AddImage` (lignes 503-509) | `=> doc.add_image_data(page, &image, x, y, width_pt)` |
| `Request::LoadImage { path: PathBuf, max_side: u32 }` | énumération `Request` (lignes 178-229) | décodage hors thread d'interface ; aucun document requis |
| `Response::ImageLoaded { path: PathBuf, result: Result<image::RgbaImage, EngineError> }` | énumération `Response` (lignes 238-286) | |
| bras de la boucle `run` | `match request` (lignes 661-822) | `Request::LoadImage { path, max_side } => { let result = sunipdf_core::signature_image::load(&path, max_side); Response::ImageLoaded { path, result } }` |

- La pose passe par `Request::Edit(EditOp::AddImageData { .. })`, donc sous instantané : **une pose = une
  entrée d'annulation**. La date passe par l'`EditOp::AddText` existant (lignes 111-117).
- **Pas de `BatchOp`** : seul le décodage d'un fichier choisi par l'utilisateur peut être long (photo de
  12 Mpx), d'où `LoadImage`. Le tracé des traits, le seuil sur une image d'au plus 1600 px et l'encodage PNG
  d'une signature rognée prennent quelques millisecondes et restent côté interface, au clic.
- `image` est déjà une dépendance du crate bureau (`apps/sunipdf-desktop/Cargo.toml`).

## 5. Interface

### 5.1 Commandes (`crates/sunipdf-core/src/command.rs`)

Trois variantes dans `Command` (lignes 5-69) et trois entrées `cmd!` dans `REGISTRY`, **après `ToolLine`**
(lignes 494-501), l'ordre du tableau étant celui des menus :

| Variante | id | Clé du libellé | Raccourci | Document requis | Menu | Panneaux d'outils |
|---|---|---|---|---|---|---|
| `SignDocument` | `sign.signature` | `menu.edit.sign` | `Some("shortcut.sign")` = `S` | oui | `Edit` | `All` (après `ToolAddImage`), `EditTab` (après `ToolLine`) |
| `SignInitials` | `sign.initials` | `menu.edit.sign_initials` | — | oui | `Edit` | `EditTab` |
| `InsertDate` | `sign.date` | `menu.edit.insert_date` | — | oui | `Edit` | `EditTab` |

- **Pas dans `ToolTab::Security`**, délibérément : ce n'est pas une fonction de sécurité.
- Le test `every_command_variant_is_registered` compte 7 commandes dans `Menu::File` (ligne 648) : ne rien
  ajouter à ce menu.
- `apps/sunipdf-desktop/src/shell.rs` : listes `ToolTab::commands` (lignes 46-145) ; `command_color`
  (lignes 150-186) : les trois dans le groupe `theme::CAT_EDIT` ; `command_icon` (lignes 246-309, `match`
  exhaustif, la compilation l'impose) : `(glyph::SIGN, "✍")`, `(glyph::INITIALS, "✎")`,
  `(glyph::CALENDAR, "📅")`.
- `apps/sunipdf-desktop/src/icons.rs`, module `glyph` (lignes 12-70) : `SIGN = "\u{E929}"`,
  `INITIALS = "\u{E70F}"`, `CALENDAR = "\u{E787}"`. Ces trois points de code sont présents dans
  `SegoeIcons.ttf` **et** `segmdl2.ttf` sur ce poste (vérifié par `GlyphTypeface.CharacterToGlyphMap` en
  PowerShell) ; leur dessin exact reste à confirmer sur capture.
- La CLI `commands` et la palette les listent sans autre changement (elles lisent `REGISTRY`).

### 5.2 Nouveau fichier `apps/sunipdf-desktop/src/sign.rs` (+ `mod sign;` dans `main.rs:3-15`)

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SignKind { #[default] Signature, Initials }

pub struct SavedSignature {
    pub path: PathBuf,
    pub kind: SignKind,
    pub image: Arc<image::DynamicImage>,
    pub thumb: egui::TextureHandle,      // au plus 320 px de côté
}

#[derive(Clone)]
pub enum Placing {
    Image { image: Arc<image::DynamicImage>, thumb: egui::TextureHandle, width_pt: f32 },
    Text(String),
}

struct Import {
    source: image::RgbaImage,            // au plus 1600 px, rendu par le worker
    small: image::RgbaImage,             // au plus 560 px de large, pour l'aperçu
    remove_bg: bool,                     // défaut : vrai si la source n'a aucun pixel transparent
    threshold: u8,                       // défaut 200, curseur 60..=250
    preview: Option<egui::TextureHandle>,// reconstruit quand remove_bg ou threshold change
}

pub struct SignState {
    pub open: Option<SignKind>,          // boîte ouverte, et sur quel onglet
    saved: Vec<SavedSignature>,          // cache global, plus récentes d'abord
    draw_mode: bool,                     // vrai = Dessiner, faux = Importer
    strokes: Vec<Vec<egui::Pos2>>,       // relatifs au coin haut-gauche du pavé, en points logiques
    drawing: bool,
    pen_rgb: [u8; 3],                    // noir (0,0,0) ou bleu (0x1E,0x3A,0x8A)
    pen_width: f32,                      // 2.5 par défaut, curseur 1.5..=6.0
    import: Option<Import>,
    import_pending: Option<PathBuf>,     // LoadImage en cours pour l'import
    save: bool,                          // « Enregistrer sur cet ordinateur », vrai par défaut
    error: Option<&'static str>,         // clé signature.error.*, affichée dans la boîte
    confirm_delete: Option<PathBuf>,
    pub placing: Option<Placing>,
    pub hover: Option<(usize, (f32, f32))>, // page et point PDF sous la souris (frame précédente)
}
```

`Default` écrit à la main (`draw_mode`, `save` vrais, `pen_width` 2.5). Champ `pub sign: SignState` dans
`SuniApp` (`app.rs:82-127`), initialisé dans `new` (`app.rs:134-201`). **Hors `EditState`** : `close_document`
remet `self.edit` à zéro (`app.rs:253`), alors que le cache des signatures est global.

Fonctions du module (toutes sans chaîne littérale d'interface) :

- `pub fn signatures_dir() -> Option<PathBuf>` : variable d'environnement `SUNIPDF_SIGNATURES_DIR` si elle
  est définie (tests et captures, même esprit que `SUNIPDF_PDFIUM_DIR` et `SUNIPDF_STARTUP_COMMANDS`), sinon
  le dossier de `Settings::path()` (`settings.rs:48-50`) + `signatures`, soit `%APPDATA%\SuniPDF\signatures`.
- `fn list_saved(dir: &Path) -> Vec<(PathBuf, SignKind)>` : `read_dir`, fichiers `signature-*.png` et
  `initials-*.png`, triés par nom décroissant, 12 au plus par type
  (`// ponytail: plafond fixe de 12 par type, images gardées décodées en mémoire`).
- Nom d'un nouveau fichier : `{signature|initials}-{millisecondes Unix}.png` (tri par nom = tri par date).
- `pub fn placement_rect(center: (f32, f32), width_pt: f32, aspect: f32, page: (f32, f32)) -> PointRect` :
  centre le rectangle sur le clic puis borne `left` à `[0, page.0 − w]` et `bottom` à `[0, page.1 − h]`.
- `fn default_width(kind: SignKind, aspect: f32) -> f32` : `min(160, 64 / aspect)` pour une signature,
  `min(64, 40 / aspect)` pour un paraphe (`aspect` = hauteur / largeur).
- `impl SuniApp` : `open_sign_dialog`, `sign_dialog`, `on_image_loaded`, `start_placing`, `cancel_placing`,
  `place_at`, `paint_placing`, `start_placing_date`.

### 5.3 La boîte « Signature manuscrite » (`sign_dialog`, via `dialog::show`)

`dialog::Spec::new("sign", titre, dark, has_icons).subtitle(tr("signature.subtitle"))
.icon(glyph::SIGN, "✍").width(600.0)` puis `dialog::show(ctx, spec, corps, pied)`
(`apps/sunipdf-desktop/src/dialog.rs:72-169`). Le corps dispose de `600 − 44 = 556` points de large
(`dialog.rs:137`). Même structure que `stamp_dialog` (`tools.rs:141-329`) : les fermetures ne font que
relever des intentions (`enum Act { Place, Use(usize), Delete(PathBuf), PickImage, Cancel }`), exécutées
après `show`.

Corps, de haut en bas :

1. Sélecteur de type : deux `ui.selectable_value` (`EGUI/ui.rs:1939`) sur `self.sign.open` ; il filtre la
   liste et fixe le type des nouveaux éléments.
2. Encadré d'avertissement, toujours visible : `Frame` (fond `p.chrome`, bordure, rayon 10, comme
   `app.rs:1716-1735`) avec l'icône `glyph::WARNING` et `tr("signature.disclaimer")`.
3. `dialog::field_label(tr("signature.saved"))` puis `ui.horizontal_wrapped` : pour chaque élément du type
   courant, une carte `ui.allocate_exact_size(vec2(150, 60), Sense::click())` (`EGUI/ui.rs:1151`), fond
   **blanc** dans les deux thèmes (encre noire illisible sur fond sombre sinon), vignette par
   `painter.image(thumb.id(), rect_ajusté, uv 0..1, Color32::WHITE)` (`EGUI/painter.rs:447-455`), bordure
   accent au survol, infobulle `signature.saved.use`, nom accessible par `resp.widget_info(||
   WidgetInfo::labeled(WidgetType::Button, true, …))` (`EGUI/response.rs:869`, `EGUI/data/output.rs:658`)
   avec la clé `a11y.signature_saved`. À droite de la carte, un petit bouton `glyph::DELETE` : premier clic →
   `confirm_delete = Some(path)` et le bouton affiche `signature.saved.delete_confirm` ; second clic →
   `Act::Delete`. Liste vide : `signature.saved.empty` en texte atténué.
4. `dialog::field_label(tr("signature.new"))` et deux `selectable_value` *Dessiner* / *Importer une image*.
5. Mode **Dessiner** — le pavé :

   ```rust
   let (resp, painter) = ui.allocate_painter(egui::vec2(556.0, 170.0), egui::Sense::DRAG);
   let rect = resp.rect;
   painter.rect_filled(rect, 8.0, egui::Color32::WHITE);          // papier, même en thème sombre
   if resp.is_pointer_button_down_on() {
       let first = !st.drawing;
       if first { st.strokes.push(Vec::new()); st.drawing = true; }
       // Tous les déplacements reçus pendant la frame (souris à 125–1000 Hz), pas seulement le dernier ;
       // sauf à la première frame du trait, où ils précèdent l'appui.
       let mut pts: Vec<egui::Pos2> = if first { Vec::new() } else {
           ui.input(|i| i.events.iter().filter_map(|e| match e {
               egui::Event::PointerMoved(p) => Some(*p), _ => None }).collect())
       };
       pts.extend(resp.interact_pointer_pos());
       let stroke = st.strokes.last_mut().expect("trait en cours");
       for p in pts {
           let q = (p.clamp(rect.min, rect.max) - rect.min).to_pos2();
           if stroke.last().is_none_or(|l| l.distance(q) > 0.5) { stroke.push(q); }
       }
   } else { st.drawing = false; }
   ```

   Sources : `allocate_painter` (`EGUI/ui.rs:1371-1376`) ; `Sense::DRAG` sans prise de focus
   (`EGUI/sense.rs:14-15`, `64-70`) ; `is_pointer_button_down_on`, vrai dès la frame d'appui, sans fenêtre de
   décision clic/glissement (`EGUI/response.rs:595-597`) ; `interact_pointer_pos` (`response.rs:546-549`) ;
   `Event::PointerMoved(Pos2)` (`EGUI/data/input/event.rs:76`), `i.events` étant déjà lu en `app.rs:649` ;
   `Pos2::clamp` (`emath-0.36.2/src/pos2.rs:193`). Tracé à l'écran : un trait de plusieurs points par
   `painter.add(egui::Shape::line(points_absolus, egui::Stroke::new(pen_width, couleur)))`
   (`EGUI/painter.rs:213`, `EPAINT/shapes/shape.rs:147-149`, `EPAINT/stroke.rs:212`), un point isolé par
   `painter.circle_filled` (`painter.rs:356`). Nom accessible du pavé : `resp.widget_info` avec
   `WidgetType::Other` et la clé `a11y.signature_pad`. Texte d'aide `signature.pad.hint` au centre tant que
   le pavé est vide. Le pavé fonctionne dans `egui::Modal` : le contenu d'une modale reçoit normalement les
   glissements (`EGUI/containers/modal.rs:77-110`), comme les `DragValue` de `stamp_dialog`.

   Sous le pavé, une rangée : libellé `signature.pad.ink`, deux pastilles (noir, bleu), `signature.pad.width`
   + `egui::Slider::new(&mut st.pen_width, 1.5..=6.0)` (`EGUI/widgets/slider.rs:128`), boutons
   `signature.pad.undo` (`strokes.pop()`) et `signature.pad.clear`.
6. Mode **Importer** : bouton `signature.import.pick` → `Act::PickImage` → `rfd::FileDialog` (mêmes filtres
   que `editor.rs:390-395`, titre `signature.import.pick_title`) → `import_pending = Some(path)` et
   `Request::LoadImage { path, max_side: 1600 }`. Pendant l'attente : `ui.spinner()` +
   `signature.import.loading`. Une fois l'image reçue : aperçu sur fond blanc (texture bâtie depuis `small`,
   passée par `remove_background(&small, threshold)` si `remove_bg`, reconstruite seulement quand un réglage
   change), case `signature.import.remove_bg`, curseur `signature.import.threshold`
   (`Slider::new(&mut threshold, 60..=250)`) actif si la case est cochée, aide
   `signature.import.threshold_hint`. C'est aussi l'alternative accessible au dessin : tout y est
   atteignable au clavier.
7. Case `signature.save` et, en texte atténué, `tr_args("signature.save_note", &[("path", …)])` avec le
   chemin renvoyé par `signatures_dir()`.
8. `self.sign.error` en `theme::ERROR` s'il est défini.

Pied : `dialog::primary(tr("signature.ok"))` → `Act::Place` ; `dialog::secondary(tr("dialog.password.cancel"))`
ou Échap → `Act::Cancel` (`dialog.rs:172-207`).

Actions :

- `Act::Place`, mode Dessiner : points × 2 (échelle de tramage), `rasterize_strokes(&pts, 1112, 340,
  pen_width * 2.0, rgb)`, `trim(&img, 6)` ; `None` → `error = Some("signature.error.empty")`, la boîte reste
  ouverte.
- `Act::Place`, mode Importer : `remove_background(&source, threshold)` si `remove_bg`, sinon la source, puis
  `trim(&img, 6)` ; mêmes erreurs.
- Suite commune : si `save`, `save_png(&img, dir.join(nom))` ; en cas d'échec, `error =
  Some("signature.error.save")` et `log::warn!` **sans chemin de contenu sensible autre que le nom du
  fichier**, la pose continue ; en cas de succès, insertion en tête de `saved`. Puis `start_placing(
  Placing::Image { image: Arc::new(DynamicImage::ImageRgba8(img)), thumb, width_pt: default_width(kind,
  aspect) })`, `open = None`, `strokes.clear()`.
- `Act::Use(i)` : `start_placing` avec l'image et la vignette de `saved[i]`, `open = None`.
- `Act::Delete(path)` : `std::fs::remove_file`, retrait de `saved`.
- `open_sign_dialog(ctx, kind)` : `open = Some(kind)`, `error = None`, `list_saved(dir)` ; pour chaque chemin
  absent du cache, `Request::LoadImage { path, max_side: 1600 }` ; retrait du cache des fichiers disparus.
- `on_image_loaded(ctx, path, result)` : si `import_pending == Some(path)` → remplir `import` (ou
  `error = Some("signature.error.image")`) ; sinon, si le chemin est dans le dossier des signatures et pas
  déjà en cache → vignette (`ColorImage::from_rgba_unmultiplied`, `EPAINT/image.rs:113` ;
  `ctx.load_texture(…, TextureOptions::LINEAR)`, `EGUI/context.rs:2390-2395`, comme `app.rs:391-401`) et
  insertion triée dans `saved` ; une erreur de lecture d'un fichier enregistré est journalisée et ignorée.

### 5.4 Mode de placement — réutilisation de `editor.rs` sans nouvel outil

Pas de nouvelle variante de `Tool` : elle imposerait un bouton dans la palette et une commande par variante
(`annotate.rs:27-56`). Un état orthogonal `self.sign.placing` suffit.

- `start_placing(p, clé_d_aide)` : `self.sign.placing = Some(p)`, `self.organize.active = false`,
  `self.status_msg = Some(tr(lang, clé_d_aide).to_string())`. L'aide doit être posée **à l'entrée** dans le
  mode : `DocumentChanged` efface `status_msg` (`app.rs:457`).
- `start_placing_date()` : `chrono::Local::now().format(tr(lang, "signature.date_format"))`
  (`CHRONO/src/offset/local/mod.rs:164`, `CHRONO/src/datetime/mod.rs:1180`), écrit par
  `write!(texte, "{}", …)` ; si l'écriture échoue (motif de format invalide dans un catalogue), repli sur
  `%Y-%m-%d`. Ne **pas** utiliser `.to_string()` : il panique quand `Display` renvoie une erreur.
- `cancel_placing()` : `placing = None`, `status_msg = None`.
- `place_at(page, pt, keep)`, où `keep` = Maj enfoncée **au moment du clic** (`ctx.input(|i| i.modifiers.shift)`
  lu dans `document_area` ; `self.edit.shift` n'est pas fiable ici : `handle_input` peut sortir avant de le
  mettre à jour, `app.rs:599-621`) :
  - `Placing::Image` : `placement_rect(pt, width_pt, aspect, taille_de_page)` puis
    `Request::Edit(EditOp::AddImageData { page, image, x: r.left, y: r.bottom, width_pt })` ;
  - `Placing::Text(t)` : `Request::Edit(EditOp::AddText { page, x: pt.0, y: pt.1, text: t, style:
    TextStyle::default() })` (12 pt, Helvetica, noir : `edit.rs:49-60`), même ancrage que l'outil texte
    (`editor.rs:374-388`) ;
  - sans Maj (`keep` faux) : `placing = None`,
    `self.annotate.tool = Tool::SelectObject`, `self.annotate.drag = None`, `self.edit.drag_mode = None`,
    `self.edit.reselect = Some((page, usize::MAX))`. À la réponse `DocumentChanged`, l'application demande
    `Request::ObjectInfo` (`app.rs:446-451`), que le worker ramène au dernier objet (`worker.rs:775-786`,
    même mécanisme que la duplication, `editor.rs:508`) : la signature apparaît sélectionnée, et tout le
    reste — poignées, déplacement, proportions verrouillées pour les images (`editor.rs:268`), flèches,
    rotation, barre d'objet — est le code existant ;
  - avec Maj : `placing` est conservé, rien d'autre ne change.
- `paint_placing(painter, page_rect, page_index, taille)` : si `placing` et `hover` concernent la page
  affichée, dessiner le fantôme — image : `painter.image(thumb.id(), rect_écran, uv 0..1,
  Color32::from_white_alpha(170))` (`ecolor-0.36.2/src/color32.rs:171`) et un filet `theme::BLUE` ; texte :
  `painter.text` (`EGUI/painter.rs:469`). Conversion points PDF → écran : même `to_screen` que
  `paint_object_selection` (`editor.rs:539-545`).

Points de branchement dans le code existant :

| Fichier:ligne | Modification |
|---|---|
| `app.rs:239-266` `close_document` | `self.sign.placing = None; self.sign.open = None;` (pas de remise à zéro complète : cache global) |
| `tabs.rs:51` `take_tab` | `self.sign.placing = None;` (le mode ne survit pas à un changement d'onglet) |
| `app.rs:327-538` `handle_responses` | bras `Response::ImageLoaded { path, result } => self.on_image_loaded(ctx, path, result)` |
| `app.rs:591-598` garde de `handle_input` | ajouter `\|\| self.sign.open.is_some()` : aucun champ de saisie n'a le focus dans cette boîte, donc sans cette garde les raccourcis à une lettre (`T`, `R`, `Suppr`, flèches…) agiraient sur le document derrière la modale |
| `app.rs`, après la ligne 671 | `if self.sign.placing.is_some() && ctx.input_mut(\|i\| i.consume_key(Modifiers::NONE, Key::Escape)) { self.cancel_placing(); }` |
| `app.rs:720-725` liste des touches | `key(Modifiers::NONE, Key::S, Command::SignDocument);` **après** Ctrl+Maj+S et Ctrl+S (`consume_key` ignore Maj et Alt en trop mais exige la même touche Ctrl : `EGUI/data/input/modifiers.rs:211-220`) |
| `app.rs:793-1011` `execute` (`match` exhaustif) | `SignDocument => self.open_sign_dialog(ctx, SignKind::Signature)`, `SignInitials => …(SignKind::Initials)`, `InsertDate => self.start_placing_date()` ; dans le bras des outils (lignes 923-942), `self.cancel_placing()` |
| `app.rs:1096` | `let mut hover_pt = None;` à côté de `drag_event` |
| `app.rs:1144` | après `paint_object_selection`, `self.paint_placing(&painter, page_rect, page_index, size);` |
| `app.rs:1182-1185` | `hover_pt = hover;` ; puis, après la fermeture du `ScrollArea`, `self.sign.hover = hover_pt.map(\|pt\| (page_index, pt));` |
| `app.rs:1187` premier bras du `match drag_event` | `Some(DragEvent::Start(pt) \| DragEvent::Click(pt)) if self.sign.placing.is_some() => self.place_at(page_index, pt, ctx.input(\|i\| i.modifiers.shift)),` — les `Move`/`Stop` suivants ne trouvent pas de glissement en cours et ne font rien |
| `app.rs:1875-1884` | `self.sign_dialog(&ctx);` avec les autres boîtes |
| `editor.rs:186` `page_cursor` | en tête : `if self.sign.placing.is_some() { return CursorIcon::Crosshair; }` |

### 5.5 Onglets, annulation, enregistrement

- Le cache des signatures et la boîte sont globaux (`SignState` n'est pas dans `TabState`,
  `tabs.rs:10-25`) ; le mode de placement est annulé au changement d'onglet et à la fermeture.
- Chaque pose est une entrée d'annulation (instantané complet, plafonds existants de 50 entrées et 512 Mo,
  `worker.rs:17-19`) ; déplacer ou redimensionner ensuite en crée d'autres, comme pour toute image.
- Rien n'est écrit dans le PDF sur disque avant *Enregistrer* (écriture atomique existante) ; l'onglet passe
  « modifié » par le chemin habituel (`app.rs:456`).

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Une sous-commande, nommée pour ce qu'elle est (une image), dans l'énumération `Command` (lignes 21-265) :

```
sunipdf-cli add-image <FILE> --image <IMAGE> [--page 1] --x <PT> --y <PT> [--width 150]
                      [--remove-background <1-255>] --out <OUT> [--password <PW>]
```

- Aide : « Place an image on a page (e.g. a scanned handwritten signature). Visual mark only: this is NOT a
  digital signature. » `--x`/`--y` : coin bas-gauche en points PDF depuis le bas-gauche de la page ;
  `--width` : largeur en points, la hauteur suit les proportions.
- Corps : `engine.open` ; `signature_image::load(&image, 4096)` ; si `--remove-background t` :
  `trim(&remove_background(&img, t), 4)` et, si `None`,
  `EngineError::Other("image is empty after background removal")` ; `doc.add_image_data(index,
  &DynamicImage::ImageRgba8(img), x, y, width)` ; `doc.save_as(&out)`. Page : `page.checked_sub(1)` comme le
  bras `Render` (lignes 587-608).
- Sortie JSON :
  `{ "out": "…", "page": 1, "x": 100.0, "y": 520.0, "width_pt": 150.0, "height_pt": 48.2,
  "background_removed": false, "digital_signature": false }`.
  Le dernier champ est constant : il évite qu'une automatisation prenne le résultat pour un document signé.
- Erreurs : codes existants (`io` → sortie 5, `page_out_of_range`, `password_required` → 3, `other`).

## 7. i18n — 40 clés nouvelles, dans `assets/i18n/fr-FR.json` **et** `en-US.json`

Préfixe `signature.*` (espace de l'agent `security-signatures`) = signature **visuelle** ; la future signature
numérique prendra un autre préfixe (`certificate.*`), pour que les deux ne se confondent jamais.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.edit.sign` | Signer (signature manuscrite)… | Sign (handwritten signature)… |
| `menu.edit.sign_initials` | Parapher (initiales)… | Add Initials… |
| `menu.edit.insert_date` | Insérer la date du jour | Insert Today's Date |
| `shortcut.sign` | S | S |
| `signature.title` | Signature manuscrite | Handwritten Signature |
| `signature.title_initials` | Paraphe | Initials |
| `signature.subtitle` | Une image apposée sur la page, sans valeur cryptographique. | An image placed on the page, with no cryptographic value. |
| `signature.disclaimer` | Signature visuelle uniquement : elle ne prouve ni l'identité du signataire ni l'intégrité du document, et quiconque modifie le fichier peut la copier ou la retirer. Ce n'est pas une signature numérique certifiée. | Visual signature only: it proves neither the signer's identity nor the document's integrity, and anyone editing the file can copy or remove it. This is not a certified digital signature. |
| `signature.kind.signature` | Signature | Signature |
| `signature.kind.initials` | Paraphe | Initials |
| `signature.saved` | Enregistrées sur cet ordinateur | Saved on this computer |
| `signature.saved.empty` | Rien d'enregistré pour l'instant. | Nothing saved yet. |
| `signature.saved.use` | Cliquer pour placer sur la page | Click to place on the page |
| `signature.saved.delete` | Supprimer de cet ordinateur | Delete from this computer |
| `signature.saved.delete_confirm` | Supprimer ? | Delete? |
| `signature.new` | Nouvelle signature | New signature |
| `signature.mode.draw` | Dessiner | Draw |
| `signature.mode.import` | Importer une image | Import an image |
| `signature.pad.hint` | Signez ici à la souris, au pavé tactile ou au stylet. | Sign here with the mouse, touchpad or pen. |
| `signature.pad.ink` | Encre | Ink |
| `signature.pad.width` | Épaisseur | Thickness |
| `signature.pad.undo` | Annuler le dernier trait | Undo last stroke |
| `signature.pad.clear` | Effacer | Clear |
| `signature.import.pick` | Choisir une image… | Choose an image… |
| `signature.import.pick_title` | Choisir l'image de la signature (PNG transparent ou photo sur fond clair) | Choose the signature image (transparent PNG or photo on a light background) |
| `signature.import.loading` | Chargement de l'image… | Loading image… |
| `signature.import.remove_bg` | Supprimer le fond clair | Remove the light background |
| `signature.import.threshold` | Seuil | Threshold |
| `signature.import.threshold_hint` | Baissez le seuil si le fond reste visible, montez-le si des traits disparaissent. | Lower the threshold if the background remains visible, raise it if strokes disappear. |
| `signature.save` | Enregistrer sur cet ordinateur pour la réutiliser | Save on this computer for reuse |
| `signature.save_note` | Stockée dans {path}, sans chiffrement, jamais envoyée sur Internet. Toute personne ayant accès à votre session Windows peut la réutiliser. | Stored in {path}, unencrypted, never sent over the Internet. Anyone with access to your Windows session can reuse it. |
| `signature.ok` | Placer sur la page | Place on Page |
| `signature.placing_hint` | Cliquez sur la page pour placer la signature. Maj+clic : placer plusieurs fois. Échap : annuler. | Click on the page to place the signature. Shift+click: place several times. Esc: cancel. |
| `signature.placing_hint_date` | Cliquez sur la page pour placer la date. Maj+clic : placer plusieurs fois. Échap : annuler. | Click on the page to place the date. Shift+click: place several times. Esc: cancel. |
| `signature.date_format` | %d/%m/%Y | %m/%d/%Y |
| `signature.error.empty` | La signature est vide : tracez un trait ou ajustez le seuil. | The signature is empty: draw a stroke or adjust the threshold. |
| `signature.error.image` | Cette image n'a pas pu être lue. | This image could not be read. |
| `signature.error.save` | La signature n'a pas pu être enregistrée sur cet ordinateur ; elle reste utilisable pour cette pose. | The signature could not be saved on this computer; it can still be placed this time. |
| `a11y.signature_pad` | Zone de dessin de la signature | Signature drawing area |
| `a11y.signature_saved` | Signature enregistrée {n} | Saved signature {n} |

Notes : `signature.title_initials` est le titre de la boîte quand le type est *Paraphe*. L'avertissement est
porté par le libellé de la commande, le sous-titre et l'encadré permanent de la boîte ; pas d'infobulle
supplémentaire. Clés réutilisées : `dialog.password.cancel`, `tools.images.filter`, `menu.edit.tool_add_text`. Le test
`placeholders_match_between_languages` (`crates/sunipdf-i18n/src/lib.rs:123-141`) impose les mêmes `{path}`
et `{n}` dans les deux langues ; `signature.date_format` n'en contient pas.

## 8. Tests

### 8.1 Unitaires, dans `crates/sunipdf-core/src/signature_image.rs` (sans `pdfium.dll`)

- `rasterize_covers_stroke_and_leaves_rest_transparent` : un trait horizontal de (10,10) à (90,10),
  `pen_px = 4`, image 100×40 → pixel (50,10) : alpha 255 et couleur d'encre ; pixel (50,30) : alpha 0 ; un
  trait d'un seul point donne une pastille (alpha 255 au centre) ; aucune panique pour un trait vide ou des
  points hors image.
- `remove_background_soft_threshold` : image à trois zones — papier (250), ombre claire (215), encre (40) ;
  seuil 200 → alpha 0, 0, 255 ; un pixel déjà transparent le reste ; la couleur de l'encre est inchangée.
- `trim_bbox_and_empty` : boîte englobante + marge, bornée à l'image ; image entièrement transparente →
  `None`.
- `png_round_trip_and_downscale` : `save_png` puis `load` dans un dossier temporaire — dimensions et alpha
  identiques, aucun `.tmp` résiduel ; une image 300×100 chargée avec `max_side = 150` revient en 150×50 ;
  une image 40×20 chargée avec `max_side = 1600` **n'est pas agrandie**.
- `load_rejects_missing_and_garbage` : fichier absent → `EngineError::Io` ; fichier texte renommé `.png` →
  `EngineError::Other`.

### 8.2 Intégration, dans `crates/sunipdf-core/tests/engine.rs` (assistants `engine()` et `fixture()`)

**`image_alpha_is_preserved_as_soft_mask`** — le jalon de la tranche. Page 1 de `two-pages.pdf` : rectangle
bleu plein de x 72..272, y 500..650 (`scripts/make_fixtures.py`, flux `c1`), rendu vérifié en
`engine.rs:84-99`.

1. Image RGBA 60×20 : colonnes 0..20 rouge opaque `(220,20,20,255)`, 20..40 rouge à demi transparent
   `(220,20,20,128)`, 40..60 **noir totalement transparent** `(0,0,0,0)` — noir pour qu'une perte de l'alpha
   se voie.
2. `doc.add_image_data(0, &DynamicImage::ImageRgba8(img), 100.0, 540.0, 150.0)` : l'image couvre
   x 100..250, y 540..590, entièrement sur le rectangle bleu.
3. `render_page(0, 1.0)`, pixels lus à `y_haut = 792 − 565` :
   - x = 125 : rouge (`r > 200`, `g < 60`, `b < 60`) ;
   - x = 175 : mélange (`80 < r < 140` et `100 < b < 170`), preuve d'un masque à 8 bits et non binaire ;
   - x = 225 : **exactement** `(0, 0, 255)`, le bleu du dessous ;
   - x = 90 (hors image) : `(0, 0, 255)`.
4. Dernier objet de `page_objects(0)` : `ObjectKind::Image`, limites ≈ (100, 540)–(250, 590) à 1 pt près.
5. Aller-retour d'annulation : `let snap = doc.snapshot()` **avant** l'ajout ; après les contrôles,
   `doc.restore(snap)` → nombre d'objets initial et pixel x = 125 redevenu bleu.
6. Persistance : nouvel ajout, `doc.restore(doc.snapshot())` puis mêmes contrôles de pixels ; `save_as` dans
   un dossier temporaire, réouverture, mêmes contrôles.
7. Édition après pose : `move_object(0, i, 0.0, -30.0)` puis `resize_object` → le pixel sous la zone
   transparente déplacée est toujours bleu.
8. Par fichier : la même image écrite en PNG par `image::save_buffer(…, image::ColorType::Rgba8)` puis
   `doc.add_image(0, &png, …)` → mêmes pixels (chemin « import d'un PNG transparent »).
9. Erreurs : `width_pt` égal à `0.0`, `f32::NAN` → `Err` ; page 9 → `Err(EngineError::PageOutOfRange(9))` ;
   le document est inchangé après chaque erreur (même nombre d'objets).

**`signature_strokes_to_page_pipeline`** : `rasterize_strokes` (un trait oblique et un point, encre
`(0x1E,0x3A,0x8A)`, 1112×340) → `trim` → `add_image_data` en zone blanche (x 300, y 100, largeur 140) →
rendu : plus de 150 pixels sombres bleutés dans la zone, et un pixel de la zone mais hors trait reste
`(255,255,255)`. **Garde-fou de taille** : `snapshot().len()` avant et après, écart inférieur à 300 000
octets (si PDFium ne compressait pas les flux d'image à l'enregistrement, l'écart dépasserait 1 Mo).

Fixtures : **aucune à générer**, les images sont bâties en mémoire et `two-pages.pdf` suffit.

### 8.3 Bureau, dans `apps/sunipdf-desktop/src/sign.rs` (`#[cfg(test)]`)

- `placement_is_centered_and_clamped` : centre sous le clic ; borné aux quatre bords ; image plus large que
  la page → `left == 0`.
- `saved_listing_filters_and_sorts` : dossier temporaire contenant deux `signature-<ms>.png`, un
  `initials-<ms>.png`, un `autre.png` et un `signature-<ms>.txt` → les signatures reviennent de la plus
  récente à la plus ancienne, le paraphe est classé à part, les deux autres fichiers sont ignorés. Le tri
  par nom suppose des horodatages de même longueur : le test utilise des millisecondes Unix réelles
  (13 chiffres) et fixe ce contrat.
- `date_never_panics` : la fonction de formatage reçoit un motif invalide (`"%Q-%"`) et renvoie le repli ISO.
- `sign_keys_exist_in_both_catalogs` : même balayage que `ui_keys_used_by_app_exist` (`app.rs:1963-1981`)
  appliqué à `include_str!("sign.rs")`.
- Déjà couverts sans rien écrire : `registry_keys_exist_in_both_catalogs` (`app.rs:1983-1992`) pour les
  libellés et le raccourci, les tests de `command.rs` pour l'unicité des identifiants.

### 8.4 Scénario manuel de captures (`scripts/dev-screenshot.ps1`, paramètres lignes 3-14)

Fenêtre 1280×780 placée en (0,0) ; les coordonnées de `-Drag` sont relatives à la fenêtre ; un « glissement »
de longueur nulle (`x,y,x,y`) vaut un clic. Les coordonnées ci-dessous sont **indicatives** : les relever
sur la capture A avant de lancer B.

```powershell
$env:SUNIPDF_SIGNATURES_DIR = "$env:TEMP\sunipdf-sign-qa"   # ne touche pas au vrai profil
Remove-Item $env:SUNIPDF_SIGNATURES_DIR -Recurse -ErrorAction SilentlyContinue
$pdf = "crates\sunipdf-core\tests\fixtures\two-pages.pdf"

# A. La boîte : avertissement lisible, liste vide, pavé blanc (à refaire en thème sombre et en anglais).
.\scripts\dev-screenshot.ps1 -Out sign-a-dialog.png -Pdf $pdf -Commands "sign.signature"

# B. Trois traits dans le pavé, clic sur « Placer sur la page », clic sur le rectangle bleu de la page.
.\scripts\dev-screenshot.ps1 -Out sign-b-placed.png -Pdf $pdf -Commands "sign.signature" `
  -Drag "470,430,560,390;560,390,640,450;640,450,760,400;<x_bouton>,<y_bouton>,<x_bouton>,<y_bouton>;520,330,520,330"
#    Attendu : signature sélectionnée (cadre et poignées), bleu visible entre les traits, onglet « modifié ».

# C. Même scénario puis Ctrl+Z : la signature a disparu.
.\scripts\dev-screenshot.ps1 -Out sign-c-undo.png -Pdf $pdf -Commands "sign.signature" -Drag "<comme B>" -KeysAfter "^z"

# D. Réutilisation : la vignette enregistrée en B apparaît ; clic dessus puis clic sur la page.
.\scripts\dev-screenshot.ps1 -Out sign-d-reuse.png -Pdf $pdf -Commands "sign.signature" `
  -Drag "<x_vignette>,<y_vignette>,<x_vignette>,<y_vignette>;640,500,640,500"

# E. Date : fantôme puis objet texte sélectionné.
.\scripts\dev-screenshot.ps1 -Out sign-e-date.png -Pdf $pdf -Commands "sign.date" -Drag "640,400,640,400"

# F. Échap annule le placement : aucun objet ajouté, curseur normal.
.\scripts\dev-screenshot.ps1 -Out sign-f-escape.png -Pdf $pdf -Commands "sign.date" -KeysAfter "{ESC}"
```

Contrôle CLI de bout en bout : `add-image` sur la fixture avec le PNG enregistré en B, puis
`sunipdf-cli render … --out page.png` et inspection de l'image.

## 9. Nouvelles dépendances

| Crate | Version | Licence | Pourquoi |
|---|---|---|---|
| `chrono` | 0.4 (0.4.45 **déjà dans `Cargo.lock`**, lignes 687-697 : dépendance de `lopdf` et de `pdfium-render`, ce dernier avec les fonctions par défaut, dont `clock`) | MIT OR Apache-2.0 (`CHRONO/Cargo.toml:39`) | Date **locale** du jour pour « Insérer la date ». La bibliothèque standard ne donne que l'heure UTC, fausse autour de minuit. Aucun crate nouveau dans l'arbre : `chrono = "0.4"` dans `[workspace.dependencies]` et `chrono = { workspace = true }` dans `apps/sunipdf-desktop/Cargo.toml`. |

Rien d'autre : tramage des traits, seuil et rognage tiennent en une centaine de lignes sur le crate `image`
déjà présent. À reporter dans `docs/THIRD_PARTY_NOTICES.md` si `chrono` n'y figure pas encore.

## 10. Risques, pièges, ordre d'implémentation, estimation

### Risques et pièges

1. **Alpha → `/SMask`** : lu dans l'amont PDFium, pas dans le binaire 7881. Levé ou confirmé par le test de
   l'étape 1 ; plan B lopdf décrit au §2.
2. **Taille des fichiers** : PDFium stocke le bitmap brut en mémoire ; on suppose qu'il compresse (Flate) les
   flux sans filtre à l'enregistrement. Non vérifiable ici : le garde-fou de taille du §8.2 tranche. S'il
   échoue : réduire l'échelle de tramage à 1,5 et faire passer l'instantané par `convert::optimize`.
3. **Raccourcis qui traversent la modale** : sans la garde ajoutée en `app.rs:591-598`, `T`, `Suppr` ou les
   flèches agiraient sur le document pendant qu'on dessine.
4. **Pavé** : utiliser `is_pointer_button_down_on()` et non `dragged()` seul, borner les points au rectangle,
   ignorer les `PointerMoved` de la première frame d'un trait (ils précèdent l'appui).
5. **`close_document` remet `self.edit` à zéro** : l'état de signature doit vivre dans `SuniApp::sign`.
6. **Motif de date** : `DelayedFormat::to_string()` panique sur un motif invalide ; passer par `write!` avec
   repli (test `date_never_panics`).
7. **Fichiers hostiles** : limites de dimensions et d'allocation dans `signature_image::load` ; ne jamais
   appeler `image::open` directement sur le fichier importé.
8. **Donnée sensible** : une signature enregistrée est réutilisable par quiconque accède au profil. Ne
   jamais journaliser de pixels ; ne journaliser que des noms de fichiers ; libellé `signature.save_note`
   explicite ; case décochable.
9. **Mémoire** : au plus 12 images par type gardées décodées (≤ 1600 px de côté, soit quelques Mo chacune au
   pire) et des vignettes de 320 px côté GPU.
10. **Pages pivotées** : pose incorrecte (limite existante, §2) — à écrire dans `KNOWN_ISSUES.md` dès cette
    tranche pour ne pas la découvrir chez l'utilisateur.
11. **Conflits de fusion** avec les tranches menées en parallèle : `command.rs` (énumération et `REGISTRY`),
    `app.rs::execute`, `shell.rs` (listes d'outils, `command_icon`), `worker.rs` (énumérations), les deux
    catalogues. L'essentiel du code est dans deux fichiers nouveaux pour limiter la surface.
12. **Test `ui_keys_used_by_app_exist`** : toute chaîne de `app.rs` en forme de clé doit exister dans les
    deux catalogues (ne pas y laisser une clé `signature.*` mal orthographiée).

### Questions ouvertes pour l'intégrateur

- Onglet d'outils dédié « Remplir et signer », commun avec la tranche formulaires ? Par défaut ce plan range
  les commandes dans « Éditer » et « Tous les outils », pour ne pas ajouter un huitième onglet à une barre
  déjà à l'étroit.
- Chiffrer les signatures enregistrées par DPAPI (`CryptProtectData`, crate `windows` déjà dans l'arbre) ?
  Par défaut : non, PNG en clair et mention explicite. Décision à prendre par ADR.
- Avertir quand le document porte déjà des signatures **numériques** : `pdfium-render` expose le compte
  (`PdfDocument::signatures()`, `PR/pdf/document.rs:320` ; `len()` → `FPDF_GetSignatureCount`,
  `PR/pdf/document/signatures.rs:31-34`). Concerne toute modification, pas seulement cette tranche : à
  porter par la tranche certificat ou par une petite tranche transversale.
- Édition sur pages pivotées : correction transversale des coordonnées (affichage → espace de page).
- Le raccourci `S` convient-il ?

### Ordre d'implémentation (chaque étape compile, sans variante morte, et laisse les tests verts)

1. **Moteur — jalon alpha.** `add_image_data` + réécriture d'`add_image` ; test
   `image_alpha_is_preserved_as_soft_mask`. `cargo test -p sunipdf-core -j 2`. *Arrêt et plan B si le masque
   n'est pas conservé.*
2. **Moteur — `signature_image.rs`** : cinq fonctions, cinq tests unitaires, test
   `signature_strokes_to_page_pipeline`.
3. **CLI `add-image`** : preuve de bout en bout scriptable (ajout puis `render`).
4. **Bureau — placement, par la date.** `chrono` ; `sign.rs` avec `SignState { placing, hover }`,
   `Placing::Text`, `start_placing_date`, `place_at`, `paint_placing`, `placement_rect` ; commande
   `InsertDate` (registre, icône, couleur, panneau, clés) ; branchements `app.rs`, `editor.rs`, `tabs.rs`.
   Captures E et F.
5. **Bureau — boîte et pavé.** `EditOp::AddImageData`, `Placing::Image`, commandes `SignDocument` et
   `SignInitials`, boîte avec avertissement et mode Dessiner (pose sans enregistrement), garde de
   `handle_input`. Captures A, B, C.
6. **Bureau — bibliothèque et import.** `Request::LoadImage` / `Response::ImageLoaded`, `signatures_dir`,
   `list_saved`, enregistrement, suppression en deux temps, mode Importer avec seuil. Capture D.
7. **Finition** : thème sombre, anglais, noms accessibles ; documentation (§11) ;
   `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace -j 2`,
   `.\scripts\check-i18n.ps1`.

### Estimation : **M**

Environ 150 lignes de moteur et autant de tests, 450 à 500 lignes pour `sign.rs`, une cinquantaine de lignes
de branchements, 40 clés × 2, une commande CLI. Les étapes 1 à 3 sont livrables seules (CLI utile sans
interface).

## 11. Documentation à mettre à jour

- `docs/PROJECT_STATUS.md` : nouvelle ligne de livraison (commandes, tests ajoutés, captures inspectées,
  nouveau total de tests — attendu 39 + 11, **à constater à l'exécution**) ; retirer « signatures » de la
  liste « non fait » uniquement pour la signature image.
- `docs/FEATURE_MATRIX.md`, section E, ligne « Signature manuscrite (image) » (ligne 79) : **Fait**, avec la
  mention « visuelle, jamais présentée comme certifiée » ; la ligne « Signature numérique CMS/PKCS#12 »
  reste *Planifié (P5)*.
- `docs/ROADMAP.md`, lignes 34 et 39 : retirer « signature image » du reste à faire.
- `docs/NEXT_TASKS.md` : suites — signature tapée, paraphe sur toutes les pages, chiffrement DPAPI,
  avertissement sur document signé numériquement, édition sur pages pivotées.
- `docs/KNOWN_ISSUES.md` : nouvelles entrées — signature visuelle sans valeur cryptographique ; stockage en
  clair dans `%APPDATA%\SuniPDF\signatures` ; seuil global ; épaisseur constante ; **pages pivotées**
  (limite de tous les outils d'édition).
- `docs/DECISIONS.md` : nouvel ADR (numéro suivant, D021 si libre) — « Signature manuscrite = objet image de
  page à masque alpha, placée par les outils d'édition ; stockage local en PNG non chiffré, enregistrement
  facultatif ; jamais dans la catégorie Sécurité ni nommée signature numérique ; pas de crate
  `sunipdf-signatures` pour cela ».
- `docs/ARCHITECTURE.md` et `CLAUDE.md` (« Modules existants ») : `sunipdf-core::signature_image`,
  `apps/sunipdf-desktop/src/sign.rs`, variable `SUNIPDF_SIGNATURES_DIR`, 21 commandes CLI.
- `docs/USER_GUIDE.fr.md` et `docs/USER_GUIDE.en.md` : section « Signer et parapher » (dessiner, importer,
  réutiliser, supprimer, date, emplacement des fichiers, **ce que cette signature ne garantit pas**) ;
  `add-image` dans « Ligne de commande » ; « Limites actuelles » garde « pas de signature numérique ».
- `docs/I18N.md` : ajouter le préfixe `signature.*` au tableau des espaces de noms.
- `docs/THIRD_PARTY_NOTICES.md` : `chrono` en dépendance directe.
- `docs/SECURITY_MODEL.md` (s'il est créé par l'agent sécurité) : paragraphe « Signature visuelle » — donnée
  personnelle stockée en clair, jamais transmise, sans valeur probante.
- `CHANGELOG.md`, section « Non publié » → « Ajouté ».
