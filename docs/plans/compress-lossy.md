# Plan — Compression avec perte (`compress-lossy`, SUNI-025, partie compression)

Statut : plan d'implémentation, aucun code écrit. Rédigé le 2026-10-14 par lecture du dépôt et des sources réelles des
bibliothèques (aucun `cargo` lancé). Effort estimé : **L** (estimation, non mesurée). Prérequis : aucun.
Nouvelles dépendances Rust : **aucune**.

Préfixes de chemins utilisés dans les citations `fichier:ligne` :

- `REG` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`
- `PR` = `REG/pdfium-render-0.9.4/src`, `LO` = `REG/lopdf-0.45.0/src`, `IMG` = `REG/image-0.25.10/src`
- `PDFIUM_H` = `C:/dev_suniworks/SuniPDF/third_party/pdfium/win-x64/include/fpdf_edit.h` (fichier `VERSION` voisin : build 7881)
- Fichiers du dépôt : chemins relatifs à `C:/dev_suniworks/SuniPDF`.

---

## 1. Objectif et comportement visible

Équivalent de l'outil **« Compress a PDF »** de PDFCraft (liste « All tools »), en local et hors ligne : réduire la taille
d'un PDF en **rééchantillonnant** les images intégrées à une résolution cible (ppp) et en les **recodant en JPEG** à une
qualité cible, avec trois préréglages **Écran / Ebook / Impression** et un mode **Personnalisé**, puis en appliquant la
passe sans perte déjà existante (`convert::optimize`, `crates/sunipdf-core/src/convert.rs:93`). La compression sans perte
actuelle (« Optimiser la taille du fichier ») reste inchangée et distincte.

Ce que voit l'utilisateur :

1. Commande **« Compresser le PDF (avec perte)… »** (`convert.compress`) : menu Convertir, panneau « Tous les outils » et
   catégorie Convertir, palette Ctrl+K, CLI `compress`. Sans document ouvert, comme les autres outils : ouverture d'un
   fichier puis exécution différée (mécanisme `after_open`, `apps/sunipdf-desktop/src/app.rs`, début de `execute`).
2. **Boîte d'options** (`dialog::show`) : 3 préréglages en boutons radio avec une ligne d'explication chacun, plus
   « Personnalisé » (résolution cible 36–600 ppp, qualité JPEG 10–95), rappel de ce qui n'est jamais modifié, avertissement
   et bouton désactivé si le document est protégé par mot de passe.
3. « Compresser… » → boîte « Enregistrer sous » native (nom suggéré `<nom>-compresse.pdf`) → tâche sur le thread moteur,
   **progression en pourcentage dans la barre d'état avec bouton Annuler**.
4. **Boîte de résultat** : « avant → après (x % de gain) », images recompressées sur le total, liste des images conservées
   avec la raison (transparence, CMJN, déjà petites…), ligne de vérification (« n pages rouvertes, écart visuel moyen
   d/255 »), boutons « Ouvrir le fichier compressé » et « Fermer ».
5. **Garanties visibles** : l'original n'est jamais modifié ni écrasé (le fichier de sortie ne peut pas être le fichier
   source) ; **aucun fichier plus gros n'est écrit** (message « Aucun gain » à la place, rien sur disque) ; une image avec
   transparence, masque, déjà petite ou déjà à la résolution cible n'est jamais touchée ; un résultat qui échoue à la
   vérification n'est pas écrit.

Le document ouvert n'est pas modifié (comme `BatchOp::Optimize`) : pas d'entrée dans l'historique d'annulation.

Correspondance PDFCraft : `docs/PDFCRAFT_AUDIT.md` ne détaille ni leurs préréglages ni leur moteur de compression (seule
la caisse `optimize` y est citée) ; **non vérifié**. Les valeurs ci-dessous sont des choix de SuniPDF, dans l'ordre de
grandeur des profils usuels de Ghostscript (72 / 150 / 300 ppp ; convention connue, non revérifiée dans cette session).

| Préréglage | Résolution cible | Qualité JPEG | Usage |
|---|---|---|---|
| Écran (`screen`) | 96 ppp | 60 | plus petit fichier, lecture à l'écran |
| Ebook (`ebook`, défaut) | 150 ppp | 75 | compromis taille / netteté |
| Impression (`print`) | 300 ppp | 88 | proche de l'original, gain plus modeste |

Règle de rééchantillonnage : une image n'est réduite que si sa résolution effective maximale (sur toutes ses utilisations)
dépasse `1,25 ×` la cible (hystérésis, évite de recompresser pour un gain marginal). Jamais d'agrandissement.

---

## 2. Faisabilité

**Verdict : partiel.** La fonction centrale est réalisable hors ligne, avec des licences permissives et sans nouvelle
dépendance, sur un périmètre d'images borné et documenté (2.2 et 2.3). Tout ce qui n'est pas dans ce périmètre est
conservé tel quel et compté dans le rapport, jamais dégradé en silence.

### 2.1 Choix de la voie : lopdf (réécriture des XObjects image), pas PDFium

Les deux voies ont été vérifiées dans les sources réelles.

**Voie PDFium** (objets image de page → `get_raw_image` → `image::imageops::resize` → `set_image`) — **rejetée** :

| Constat | Preuve |
|---|---|
| `get_raw_image` ignore filtres, **masque d'image** et matrice | `PR/pdf/document/page/object/image.rs:268-291` ; `PR/bindings.rs:8858-8867` ; `PDFIUM_H:807-811` (« does not take the associated image mask into account. It also ignores the matrix ») |
| Aucune API PDFium ne dit si une image a un /SMask ou /Mask, ni son numéro d'objet : `FPDF_IMAGEOBJ_METADATA` = largeur, hauteur, ppp, bits/pixel, espace colorimétrique, id de contenu marqué. On ne peut donc pas « ne pas toucher aux images transparentes » : un `set_image` détruirait la transparence en silence | `PDFIUM_H:77-93` ; `PR/pdf/document/page/object/image.rs:742-781` (`get_raw_metadata`) |
| `set_image` convertit toujours en RGBA puis BGRA et appelle `FPDFImageObj_SetBitmap` avec `pages = NULL` (le cache d'image des pages déjà chargées n'est pas invalidé) | `PR/pdf/document/page/object/image.rs:686-741` ; `PDFIUM_H:793-805` |
| **Mesuré** avec le `pdfium.dll` build 7881 du dépôt (sonde ctypes jetable, hors dépôt, 2026-10-14, qui reproduit l'appel de `set_image` : bitmap BGRA créé avec `FPDFBitmap_CreateEx`, rempli, puis `FPDFImageObj_SetBitmap` ; la sonde n'est **pas** passée par pdfium-render) : une image posée via un bitmap BGRA produit `/ColorSpace /DeviceRGB /Filter /FlateDecode` **plus un second flux `/SMask` DeviceGray Flate** (opaque) ; via BGRx ou BGR, seulement le flux Flate. **Jamais de JPEG** : la voie PDFium ne fait pas de compression avec perte, elle ré-encode en Flate sans perte (souvent plus gros que le JPEG d'origine) et ajoute un SMask | sortie de la sonde : `<</BitsPerComponent 8/ColorSpace/DeviceRGB/Filter/FlateDecode/Height 64/Length 38/SMask 5 0 R /Subtype/Image/Type/XObject/Width 64>>` (BGRA) et la même sans `/SMask` (BGRx, BGR) |
| Produire du JPEG exige de créer un nouvel objet (`new_from_jpeg_reader` → `FPDFImageObj_LoadJpegFileInline`), puis de remplacer l'ancien dans la page en recopiant matrice, z-ordre, découpe et mode de fusion | `PR/pdf/document/page/object/image.rs:167-198` ; `PR/bindings.rs:8788` |
| Chaque page modifiée est réécrite par `regenerate_content` (flux de contenu régénéré entièrement par PDFium) : risque de fidélité sur toute la page | `PR/pdf/document/page.rs:942-971` |
| Une image partagée par N pages est dupliquée (un flux par objet image) → taille en hausse | **non vérifié** dans les sources C++ (indisponibles hors ligne) ; cohérent avec la sonde |
| Découverte collatérale : `Engine::images_to_pdf` (`convert.rs:17-48`, `new_with_size` → `set_image`) produit, par déduction depuis la sonde (`image::open` → `to_rgba8` → BGRA), ce motif Flate + SMask opaque ; à confirmer en relisant un PDF réellement produit par `images2pdf` (premier test de l'étape 4). La compression doit donc savoir **retirer un SMask entièrement opaque** | même sonde |

**Voie lopdf** (réécriture directe des flux `/Subtype /Image`) — **retenue**. Elle suit le patron déjà utilisé par
`optimize`, `protect`, `set_metadata` (octets → lopdf → octets, résultat relu par PDFium, D016), conserve **tout** le
dictionnaire de l'image (`/ColorSpace`, `/Intent`, `/OC`, `/StructParent`…), ne touche pas aux flux de contenu, partage
naturellement une image entre pages, voit `/SMask` et `/Mask` (`Dictionary::get`, `LO/object.rs:352`) et ne dépend pas de
PDFium pour la transformation (testable sans DLL, déplaçable hors thread moteur plus tard).

Inconvénients assumés : (a) mesurer soi-même la résolution effective de chaque image (interprétation de `q`/`Q`/`cm`/`Do`
et des formulaires imbriqués) ; (b) lopdf réécrit tout le fichier : linéarisation perdue, signatures numériques invalidées,
flux d'objets éclatés (le fichier peut grossir) — déjà vrai pour `optimize`, couvert ici par la règle « jamais plus gros » ;
(c) décoder JPEG/Flate nous-mêmes (crate `image`, déjà présente).

### 2.2 Périmètre : ce qui est fait

- Images `/Subtype /Image` avec **un seul filtre** parmi : aucun, `FlateDecode` (prédicteurs PNG et TIFF gérés par lopdf),
  `LZWDecode` (échantillons bruts), ou `DCTDecode` (JPEG).
- Espaces colorimétriques à **1 composante** (`DeviceGray`, `CalGray`, `ICCBased /N 1`) ou **3 composantes** (`DeviceRGB`,
  `CalRGB`, `ICCBased /N 3`), 8 bits par composante, sans tableau `/Decode`. L'objet `/ColorSpace` d'origine est
  **conservé tel quel** : les données restent dans le même espace, le profil ICC est préservé.
- Images avec `/SMask` **entièrement opaque** (tous les échantillons à 255) : SMask retiré (gain, aucun effet visuel). Cas
  des PDF créés par « Créer un PDF depuis des images ».
- Sources Flate « photographiques » → JPEG ; sources JPEG → rééchantillonnées puis ré-encodées en JPEG.
- Sortie : le dictionnaire reçoit `/Width`, `/Height`, `/Filter /DCTDecode` et perd `/DecodeParms`.

### 2.3 Périmètre : ce qui est volontairement exclu (à documenter comme limites)

| Exclu | Pourquoi | Comportement |
|---|---|---|
| Images à **transparence réelle** (`/SMask` non opaque, `/Mask` flux ou clé de couleur) | rééchantillonner l'image sans son masque désaligne la transparence ; traiter les deux ensemble est une extension | conservées, raison `transparency` |
| `/ImageMask true` (masques au trait) | 1 bit, JPEG détruirait les contours | conservées, raison `stencil` |
| **1 bit par composante** (scans noir et blanc) et 16 bits | JPEG dégrade le texte binaire et le grossit ; le rééchantillonnage binaire demande un autre algorithme | conservées, raison `bitdepth` |
| **CMJN** (`DeviceCMYK`, `ICCBased /N 4`, JPEG Adobe inversé) | `image` décode un JPEG CMJN **en le convertissant en RGB** (`IMG/codecs/jpeg/decoder.rs:198-205`, « the rest is not supported by `image` so it will be converted to RGB ») et son encodeur n'écrit que `L8` et `Rgb8` (`IMG/codecs/jpeg/encoder.rs:479-531`) : changer d'espace colorimétrique à l'insu de l'utilisateur est inacceptable | conservées, raison `colorspace` |
| `Indexed`, `Separation`, `DeviceN`, `Lab`, `Pattern`, `/Decode` non identité | recoder en JPEG change les couleurs ou exige une conversion | conservées, raison `colorspace` |
| `JPXDecode`, `JBIG2Decode`, `CCITTFaxDecode`, chaînes de plusieurs filtres, `/DecodeParms` indirect ou tableau | pas de décodeur permissif hors ligne dans l'arbre actuel (lopdf renvoie `Unimplemented` pour tout filtre hors Flate/Brotli/LZW/ASCII85/ASCIIHex/RunLength : `LO/object.rs:938-947`) ; un prédicteur ignoré donnerait des pixels faux **de longueur correcte** (`LO/object.rs:920-922` ne lit `/DecodeParms` que direct) | conservées, raison `filter` |
| Images « graphiques » Flate (aplats, schémas, captures : peu de couleurs distinctes) | JPEG les dégrade et les grossit | conservées, raison `graphic` (extension possible : réduction seule en Flate, étape 9) |
| Images sans opérateur `Do` mesurable (annotations, motifs, polices Type 3, contenu illisible) | résolution effective inconnue | conservées, raison `unmeasured` |
| Images inline (`BI … EI`) | ce ne sont pas des XObjects | non traitées |
| JPEG existants **non réduits** | ré-encoder au même format dégrade sans gain prévisible | conservés, raison `resolution` |
| **Documents chiffrés** | `security::load` / `optimize` retirent le chiffrement en silence ; une compression ne doit pas supprimer une protection | refus `EngineError::SecurityRestricted` ; l'utilisateur retire la protection (menu Sécurité), compresse, protège de nouveau |
| Estimation du gain avant exécution, objectif « N Mo », OCR, fusion d'images identiques | hors tranche | roadmap |

---

## 3. API moteur (`sunipdf-core`)

### 3.1 Sources vérifiées (chaque appel utilisé plus bas figure ici)

| API | Source |
|---|---|
| `lopdf::Document::load_mem(&[u8]) -> Result<Document>` | `LO/reader.rs:95` |
| `Document::objects: BTreeMap<ObjectId, Object>`, `trailer`, `max_id` (champs publics) | `LO/document.rs:28-37` |
| `Document::dereference`, `get_object`, `get_dictionary`, `get_dict_in_dict` | `LO/document.rs:147, 165, 202, 212` |
| `Document::page_iter()` / `get_pages()` | `LO/document.rs:533-539` |
| `Document::get_page_contents(page_id)`, `get_page_content_with_limit(page_id, max) -> Result<Vec<u8>>` (borné contre les bombes de décompression) | `LO/document.rs:542, 631` |
| `Document::get_page_resources` ne remonte **pas** un `/Resources` **direct** hérité d'un parent (seulement des références) → écrire notre propre remontée par `/Parent` | `LO/document.rs:666-696` |
| `lopdf::content::Content::decode(&[u8]) -> Result<Content<Vec<Operation>>>`, `Operation { operator: String, operands: Vec<Object> }` | `LO/parser_aux.rs:20-23` ; `LO/content.rs:7-10` |
| `Stream { dict, content, allows_compression, start_position }`, `Stream::new` (fixe `/Length`), `filters()`, `set_content` (met à jour `/Length`), `compress()` (Flate si `/Filter` absent et gain > 19 octets) | `LO/object.rs:24-35, 780, 807, 822, 857` |
| `Stream::decompressed_content_with_limit(max) -> Result<Vec<u8>>`. **Pièges** : un flux Flate corrompu est renvoyé **tronqué sans erreur** (`warn!` puis repli deflate brut) ; `/DecodeParms` n'est lu que s'il est un dictionnaire direct | `LO/object.rs:898, 1051-1085, 920-922` |
| Prédicteurs Flate/LZW : TIFF 2 et PNG 10-15 appliqués par lopdf | `LO/object.rs:1215-1245` |
| `Dictionary::get / get_deref / has / set / remove / iter`, `Object::as_name / as_i64 / as_float / as_dict / as_stream / as_array / as_reference` ; `From<&str> for Object` produit un **Name** | `LO/object.rs:145-290, 348-388, 106-110` |
| `Document::compress()`, `delete_zero_length_streams()`, `prune_objects()`, `renumber_objects()` | `LO/processor.rs:22, 113, 66, 135` |
| `Document::save_to(&mut Vec<u8>)` (xref classique) ; `save_modern` (flux d'objets + xref stream) existe | `LO/writer.rs:22, 57` |
| `image::ImageReader::with_format(Cursor<&[u8]>, ImageFormat::Jpeg)`, `.limits(Limits)`, `.decode() -> DynamicImage` ; `Limits { max_image_width, max_image_height, max_alloc }` (dimensions « strict ») | `IMG/io/image_reader_type.rs:101, 137, 311` ; `IMG/io/limits.rs:33-43` |
| `DynamicImage::into_rgb8()`, `into_luma8()`, `color()` | `IMG/images/dynimage.rs:382, 454, 750` |
| `image::imageops::resize(&I, nw, nh, FilterType::Lanczos3)` | `IMG/imageops/sample.rs:92-107, 964-990` |
| `image::codecs::jpeg::JpegEncoder::new_with_quality(W: Write, quality: u8)` ; `.encode(&[u8], w, h, ExtendedColorType)` : **`L8` et `Rgb8` seulement**, **panique si `len != w*h*bpp`**, sous-échantillonnage 4:2:2 imposé, dimensions 1..=65535 | `IMG/codecs/jpeg/encoder.rs:398, 468-531, 533-560` |
| `image::{ExtendedColorType, RgbImage, GrayImage}` réexportés ; fonctionnalité `jpeg` activée dans `Cargo.toml` racine (`image = { default-features = false, features = ["png","jpeg",…] }`) | `IMG/lib.rs:129, 139-146` |
| `Engine::open_bytes(Vec<u8>, Option<&str>, &Path)`, `Document::{page_count, page_size, render_page, snapshot, info}` | `crates/sunipdf-core/src/lib.rs:143, 211, 215, 224, 207` ; `pages.rs:246` |
| Réutilisables dans le crate : `security::save(lopdf::Document) -> Result<Vec<u8>>` (`pub(crate)`), `security::lopdf_err` (privée : en redéfinir une identique dans `compress.rs`), `write_atomic` | `security.rs:407, 429` ; `pages.rs:63` |

Versions verrouillées (`Cargo.lock`) : lopdf 0.45.0, image 0.25.10, zune-jpeg 0.5.15, pdfium-render 0.9.4, flate2 1.1.10
(transitive via lopdf, non utilisée directement).

### 3.2 Nouveau module `crates/sunipdf-core/src/compress.rs`

Déclaré dans `lib.rs` (`pub mod compress;`, `pub mod verify;`) avec ré-exports
`pub use compress::{CompressOptions, CompressOutcome, CompressPreset, CompressReport, SkipReason};` et
`pub use verify::Verification;`.

```rust
use std::ops::ControlFlow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressPreset { Screen, Ebook, Print }
impl CompressPreset {
    pub const ALL: [CompressPreset; 3] = [Self::Screen, Self::Ebook, Self::Print];
    pub fn id(self) -> &'static str;                 // "screen" | "ebook" | "print"
    pub fn from_id(id: &str) -> Option<Self>;        // même patron que StampPosition::from_id (stamp.rs:42)
    pub fn options(self) -> CompressOptions;         // 96/60, 150/75, 300/88
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressOptions {
    pub target_dpi: f32,          // 36.0..=600.0
    pub jpeg_quality: u8,         // 10..=95
    pub resample_threshold: f32,  // 1.25 : réduire seulement au-delà de target_dpi * seuil
    pub min_stream_bytes: usize,  // 16 * 1024 : en dessous, image « déjà petite »
    pub min_gain: f32,            // 0.10 : remplacer seulement si nouveau <= ancien * (1 - min_gain)
    pub max_pixels: u64,          // 100_000_000 : au-delà, raison TooLarge
    pub lossless_to_jpeg: bool,   // true : image Flate photographique recodée en JPEG même sans réduction
}
impl CompressOptions { pub fn validate(&self) -> Result<(), EngineError>; } // Other("invalid compression options")
impl Default for CompressOptions { /* = CompressPreset::Ebook.options() */ }

/// Pourquoi une image est laissée telle quelle. L'ordre des variantes = ordre de priorité des contrôles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    Unmeasured, Small, StencilMask, Transparency, ColorSpace, BitDepth, Filter,
    TooLarge, Corrupt, Graphic, AtTargetResolution, NotSmaller,
}
impl SkipReason { pub fn code(self) -> &'static str; }
// "unmeasured" "small" "stencil" "transparency" "colorspace" "bitdepth" "filter"
// "too_large" "corrupt" "graphic" "resolution" "no_gain"

#[derive(Debug, Clone, PartialEq)]
pub struct CompressReport {
    pub before: u64,                    // taille de l'entrée (octets)
    pub after: u64,                     // taille du candidat produit (même s'il est plus gros)
    pub pages: usize,
    pub images_found: usize,            // flux /Subtype /Image distincts
    pub images_recompressed: usize,
    pub opaque_masks_dropped: usize,
    pub image_bytes_before: u64,        // somme des flux d'images recompressées
    pub image_bytes_after: u64,
    pub kept: Vec<(SkipReason, usize)>, // regroupé, trié par SkipReason
}

pub enum CompressOutcome {
    /// `bytes = Some` seulement si `after < before` ; None = « aucun gain », ne rien écrire.
    Done { bytes: Option<Vec<u8>>, report: CompressReport },
    Cancelled,
}

/// `progress(done, total)` est appelé avant chaque image ; `ControlFlow::Break(())` annule proprement.
pub fn compress_pdf(
    input: &[u8],
    options: &CompressOptions,
    progress: &mut dyn FnMut(usize, usize) -> ControlFlow<()>,
) -> Result<CompressOutcome, EngineError>;

/// Vrai si `a` et `b` désignent le même fichier (canonicalize ; si `b` n'existe pas encore : parent canonique + nom).
pub fn same_file(a: &Path, b: &Path) -> bool;
```

Pas de paramètre mot de passe : un document chiffré est refusé (2.3). Pas de `compress_file` : la CLI et le worker
ont besoin des octets pour la vérification (YAGNI).

### 3.3 Algorithme de `compress_pdf` (étapes et pièges)

0. `options.validate()?`.
1. `let mut doc = lopdf::Document::load_mem(input).map_err(lopdf_err)?` (`LO/reader.rs:95`). Si
   `doc.trailer.has(b"Encrypt")` (`LO/object.rs:348`, plus robuste que `is_encrypted()` qui exige une référence,
   `LO/document.rs:262-270`) → `Err(EngineError::SecurityRestricted)`.
2. **Mesure des usages** (`fn measure(&doc) -> HashMap<ObjectId, Usage>`, `Usage { max_w_in: f32, max_h_in: f32 }`), sans PDFium :
   - Pour chaque page de `doc.page_iter()` : ressources par remontée `/Resources` puis `/Parent` (profondeur ≤ 64,
     dictionnaire direct ou référence via `doc.dereference`) ; `/UserUnit` (défaut 1.0) ; contenu =
     `doc.get_page_content_with_limit(page_id, 256 << 20)` puis `Content::decode`. Échec (bombe, syntaxe) → page non mesurée.
   - Parcours des opérations avec pile de matrices : `q` empile, `Q` dépile (pile vide ignorée), `cm` fait
     `ctm = mul(M, ctm)` (six opérandes via `as_float`), `Do` avec un nom : `Resources/XObject/<nom>` → `doc.get_object(id)`
     → flux ; `Subtype /Image` → enregistrer l'usage ; `Subtype /Form` → récursion avec
     `ctm = mul(form.Matrix /* identité par défaut */, ctm)`, ressources du formulaire sinon héritées, **profondeur ≤ 8,
     budget global de 2 000 000 d'opérations**, détection de cycle par pile des `ObjectId` de formulaires.
   - Convention PDF : le carré unité de l'image est envoyé par le CTM ; largeur affichée (unités utilisateur)
     `= hypot(ctm.a, ctm.b)`, hauteur `= hypot(ctm.c, ctm.d)` (valable pour rotations et cisaillements). En pouces :
     `unités * user_unit / 72`. On garde le **maximum par axe sur toutes les utilisations**. Usages de taille nulle ou
     non finie ignorés.
   ```rust
   type Mat = [f32; 6]; // a b c d e f
   fn mul(m: Mat, n: Mat) -> Mat { // m puis n : nouveau CTM = M × CTM (convention PDF)
       [m[0]*n[0] + m[1]*n[2], m[0]*n[1] + m[1]*n[3],
        m[2]*n[0] + m[3]*n[2], m[2]*n[1] + m[3]*n[3],
        m[4]*n[0] + m[5]*n[2] + n[4], m[4]*n[1] + m[5]*n[3] + n[5]]
   }
   ```
3. **Candidats** : tous les flux de `doc.objects` avec `/Subtype /Image`, par `ObjectId` croissant (déterminisme).
   `total` = leur nombre ; `progress(i, total)` avant chacun (`Break` → `Ok(Cancelled)`, rien n'est produit).
4. **Classification** (lecture seule, `fn classify(&doc, id, &Stream, Option<&Usage>, &opts) -> Result<Job, SkipReason>`).
   La première raison rencontrée gagne ; cet ordre est testé :
   1. pas d'usage mesuré → `Unmeasured` ;
   2. `stream.content.len() < min_stream_bytes` ou largeur/hauteur < 16 → `Small` ;
   3. `/ImageMask true` → `StencilMask` ;
   4. `/Mask` présent (flux ou tableau) → `Transparency` ; `/SMask` présent : le résoudre ; s'il n'est **pas** un flux
      Image DeviceGray 8 bits sans `/Matte`, avec `/Decode` absent ou `[0 1]`, filtre aucun/Flate, décodé en exactement
      `w_s*h_s` octets **tous égaux à 255** → `Transparency` ; sinon `drop_smask = true` ;
   5. `/ColorSpace` → nombre de composantes (`fn components(&doc, &Object) -> Option<u8>` : nom `DeviceGray`/`DeviceRGB` ;
      tableau commençant par `CalGray`/`CalRGB` ; `[/ICCBased ref]` → `/N` du flux ; tout le reste `None`) ; `None` ou
      `/Decode` non identité → `ColorSpace` ;
   6. `/BitsPerComponent` ≠ 8 (ignoré pour un JPEG) → `BitDepth` ;
   7. filtre (`Stream::filters()`, `LO/object.rs:807`) : accepté = aucun, `[FlateDecode]`, `[LZWDecode]` → source brute ;
      `[DCTDecode]` → JPEG ; `/DecodeParms` doit être absent ou **dictionnaire direct** (sinon `Filter`) ; avec `/Predictor`
      ≥ 2 : `/Columns == Width` et `/Colors == composantes` exigés (sinon `Corrupt`) ; DCT avec `/ColorTransform` ≠ 1 en
      3 composantes → `Filter` ; tout autre filtre → `Filter` ;
   8. `w*h > max_pixels` → `TooLarge`.
5. **Dimensions cibles** (`fn target_dims`) : `eff_dpi = max(w / max_w_in, h / max_h_in)`. Si
   `eff_dpi > target_dpi * resample_threshold` : `new_w = round(max_w_in * target_dpi)`, `new_h = round(max_h_in * target_dpi)`,
   bornées à `[min(8, w), w]` et `[min(8, h), h]`. Sinon dimensions inchangées, autorisé seulement pour une source brute
   photographique avec `lossless_to_jpeg` (conversion Flate → JPEG) ; sinon `AtTargetResolution`.
6. **Décodage** (`fn decode(&Stream, &Job) -> Result<Pixels, SkipReason>`) :
   - JPEG : `ImageReader::with_format(Cursor::new(&stream.content), ImageFormat::Jpeg)` avec
     `limits(Limits { max_image_width: Some(w), max_image_height: Some(h), max_alloc: Some(512 << 20) })` (un en-tête JPEG
     plus grand que les dimensions déclarées échoue : protection contre les bombes), `.decode()` ; 3 composantes →
     `into_rgb8()`, 1 → `into_luma8()` ; dimensions décodées ≠ `/Width`/`/Height`, ou `color()` incohérent avec l'espace
     (ex. JPEG gris déclaré RGB) → `Corrupt`.
   - Brut : `stream.decompressed_content_with_limit(w*h*comps + 65_536)` ; **la longueur doit être ≥ `w*h*comps`** (on tronque
     l'excédent), sinon `Corrupt` (lopdf renvoie du contenu tronqué sans erreur, `LO/object.rs:1051-1085`).
   - Heuristique photo / graphique (source brute seulement) : échantillon régulier de ≤ 16 384 pixels ; RGB : couleurs
     distinctes après quantification à 5 bits par canal, **photo si > 512** ; gris : niveaux distincts, **photo si ≥ 64** ;
     sinon `Graphic`. Seuils initiaux **non calibrés sur un corpus réel** : à ajuster avec les fixtures (étape 4) et à consigner.
7. **Recodage** : `imageops::resize(&buf, new_w, new_h, FilterType::Lanczos3)` si les dimensions changent ;
   `JpegEncoder::new_with_quality(&mut Vec<u8>, quality).encode(raw, new_w, new_h, ExtendedColorType::Rgb8 | L8)` (longueur
   garantie par construction, l'encodeur panique sinon). Accepter seulement si `nouveau <= ancien * (1 - min_gain)`
   (octets du flux), sinon `NotSmaller`.
8. **Application** (mutation `&mut doc`, une image à la fois) : `stream.set_content(jpeg)` ;
   `dict.set("Width", new_w as i64)` ; `dict.set("Height", new_h as i64)` ; `dict.set("Filter", "DCTDecode")` (un `&str`
   devient un Name, `LO/object.rs:106`) ; `dict.remove(b"DecodeParms")` ; si `drop_smask` : `dict.remove(b"SMask")`.
   `/ColorSpace`, `/BitsPerComponent 8` et toutes les autres clés restent. Cumuler `image_bytes_before/after`.
9. **Passe sans perte**, identique à `convert::optimize` (`convert.rs:112-119`) : `doc.compress()`,
   `delete_zero_length_streams()`, `prune_objects()` (supprime les SMask devenus orphelins), `renumber_objects()`, puis
   `security::save(doc)` (`security.rs:429`).
10. **Règle « jamais plus gros »** : `bytes = Some(out)` seulement si `out.len() < input.len()`. Le rapport contient
    toujours les tailles réelles. Le moteur n'écrit jamais lui-même : l'appelant écrit via `write_atomic`.

Gestion d'erreurs : erreurs lopdf → `EngineError::Other("lopdf: …")` (comme `security.rs:407`) ; chiffré →
`SecurityRestricted` ; options invalides → `Other` ; une image qui échoue ne fait **jamais** échouer le document (raison
`Corrupt`, image intacte) ; aucune panique possible (le tampon passé à `JpegEncoder::encode` a une longueur vérifiée).

Régénération du contenu / instantané : **aucun impact**. Cette voie ne touche ni aux flux de contenu ni au document PDFium
ouvert ; `compress_pdf` travaille sur les octets de `Document::snapshot()` (état courant, y compris les modifications non
enregistrées, comme `BatchOp::Optimize`). Pas de `restore`, pas d'entrée d'annulation.

### 3.4 Vérification de sortie : nouveau module `crates/sunipdf-core/src/verify.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Verification {
    pub pages: usize,
    pub sampled: usize,            // pages comparées au rendu (première, médiane, dernière)
    pub max_mean_abs_diff: f32,    // 0..255 : moyenne de (|ΔR|+|ΔG|+|ΔB|)/3 par page, maximum sur les pages
}
impl Engine {
    /// Rouvre `compressed` avec PDFium et le compare à `original` (document ouvert, même état que l'instantané).
    pub fn verify_compressed(&self, original: &Document<'_>, compressed: &[u8])
        -> Result<Verification, EngineError>;   // Err(EngineError::VerificationFailed) si un contrôle échoue
}
```

Contrôles : (1) `self.open_bytes(compressed.to_vec(), None, &original.info().path)` réussit ; (2) même nombre de pages ;
(3) mêmes tailles de page (tolérance 0,01 pt) ; (4) rendu des pages échantillonnées à `scale = 0.5` des deux côtés
(`Document::render_page`, `lib.rs:224`) et écart moyen ≤ `VERIFY_MAX_MEAN_ABS_DIFF`. Les détails d'un échec partent dans
`log::warn!` ; l'erreur typée n'a pas de charge utile.

**Seuil `VERIFY_MAX_MEAN_ABS_DIFF` : valeur initiale 20,0 à calibrer par mesure à l'étape 4** (écart observé en JPEG q60 sur
la fixture « photo », écart d'une image remplacée par du noir), puis seuil fixé avec marge et mesures consignées dans
`docs/PROJECT_STATUS.md`. Aucun chiffre non mesuré ne doit être présenté.

Nouvelle variante d'erreur (impact transversal, voir 5.6) : `EngineError::VerificationFailed`
(`#[error("output verification failed")]`, `code() = "verification_failed"`).

**Vérification indépendante qpdf** (outil de développement, jamais embarqué), même fichier `verify.rs` :

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QpdfStatus { Unavailable, Ok, Warnings(String), Errors(String) }
pub fn find_qpdf() -> Option<PathBuf>;   // SUNIPDF_QPDF, puis third_party/qpdf/win-<arch>/bin/qpdf.exe
                                         // (même schéma que default_library_dirs, lib.rs:83-109), puis PATH
pub fn qpdf_check(path: &Path) -> Result<QpdfStatus, EngineError>;   // std::process::Command, aucune dépendance
```

`qpdf --check <fichier>` ; codes de sortie documentés : 0 = aucun problème, 2 = erreurs, 3 = avertissements seuls
(`qpdf.readthedocs.io/en/stable/cli.html`, consultée le 2026-10-14). `--check` vérifie « la structure, le chiffrement, la
linéarisation et l'encodage des flux » mais **pas les pixels** et la documentation prévient qu'un fichier sans erreur
« peut encore avoir des erreurs dans le contenu des flux » : il complète la relecture PDFium, il ne la remplace pas. Sous
Windows : `std::os::windows::process::CommandExt::creation_flags(0x0800_0000)` (CREATE_NO_WINDOW). Aucun accès réseau. Le
bureau n'appelle pas qpdf (sous-processus en production hors périmètre) ; seuls les tests et la CLI (`--qpdf-check`)
l'utilisent.

#### Script `scripts/fetch-qpdf.ps1` (facultatif, patron de `fetch-pdfium.ps1`)

Licence qpdf : **Apache License 2.0** (`qpdf.readthedocs.io/en/stable/license.html`, consultée le 2026-10-14). Jamais copié
dans `dist/`, ni dans le MSI, ni dans le ZIP portable ; `package-portable.ps1` et `package-msi.ps1` ne le référencent
pas. Version vérifiée : v12.4.2 (publiée le 2026-09-27, API GitHub). Archives : `qpdf-12.4.2-msvc64.zip`,
`qpdf-12.4.2-msvc-arm64.zip`, somme `qpdf-12.4.2.sha256`. Contenu du zip x64 (inspecté localement) :
`qpdf-12.4.2-msvc64/bin/qpdf.exe` + DLL (`qpdf30.dll`, runtime MSVC). SHA256 publiés, **épinglés dans le script** :

| Archive | SHA256 |
|---|---|
| `qpdf-12.4.2-msvc64.zip` | `db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d` (**recalculé localement après téléchargement : identique**) |
| `qpdf-12.4.2-msvc-arm64.zip` | `2b5d7740a2513d8a841116692dc2cf7efe83f35262386ebab18bed35fbf15ab1` (publié, **non téléchargé** ici) |

```powershell
# scripts/fetch-qpdf.ps1 — outil de vérification de développement (Apache-2.0), jamais livré avec l'application.
param([ValidateSet("x64","arm64")] [string]$Arch = "x64", [string]$Version = "12.4.2", [switch]$Force)
$ErrorActionPreference = "Stop"
$pins = @{ "12.4.2-x64"   = "db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d"
           "12.4.2-arm64" = "2b5d7740a2513d8a841116692dc2cf7efe83f35262386ebab18bed35fbf15ab1" }
$flavor = if ($Arch -eq "arm64") { "msvc-arm64" } else { "msvc64" }
$root = Split-Path -Parent $PSScriptRoot
$dir  = Join-Path $root "third_party\qpdf\win-$Arch"
$exe  = Join-Path $dir "bin\qpdf.exe"
if ((Test-Path $exe) -and -not $Force) { Write-Host "qpdf déjà présent : $exe"; exit 0 }
$expected = $pins["$Version-$Arch"]
if (-not $expected) { throw "Version $Version non épinglée : ajouter son SHA256 dans `$pins." }
New-Item -ItemType Directory -Force $dir | Out-Null
$zip = Join-Path $dir "qpdf.zip"
Invoke-WebRequest -UseBasicParsing -OutFile $zip -Uri "https://github.com/qpdf/qpdf/releases/download/v$Version/qpdf-$Version-$flavor.zip"
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
if ($hash -ne $expected) { Remove-Item $zip; throw "SHA256 inattendu : $hash" }
Expand-Archive $zip -DestinationPath $dir -Force; Remove-Item $zip
$inner = Join-Path $dir "qpdf-$Version-$flavor"
Move-Item (Join-Path $inner "*") $dir -Force; Remove-Item $inner -Recurse
& $exe --version
```

À ajouter : `/third_party/qpdf/` dans `.gitignore` ; un pas CI facultatif dans `.github/workflows/ci.yml`
(`.\scripts\fetch-qpdf.ps1`, puis `SUNIPDF_REQUIRE_QPDF=1` pour les tests ; **non vérifiable sur ce poste**, voir
KNOWN_ISSUES A10) ; une ligne « outil de développement, Apache-2.0, non redistribué » dans
`docs/THIRD_PARTY_NOTICES.md` et dans la ligne qpdf de `docs/TECH_STACK.md`.

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

Aucun nouveau `EditOp` (voir la décision en 5.7) ; la compression est une **tâche de fichiers** de plus, annulable et avec
progression. Elle tourne sur le thread moteur (elle y passe de toute façon par `Document::snapshot()` et la vérification
PDFium) ; pendant qu'elle tourne, les rendus attendent (limite documentée, 10). La progression et l'annulation passent par
deux atomiques partagés, sans nouveau canal :

```rust
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use sunipdf_core::compress::{CompressOptions, CompressOutcome, CompressReport};
use sunipdf_core::Verification;

pub enum BatchOp {
    /* … variantes existantes … */
    Compress {
        out: PathBuf,
        options: CompressOptions,
        cancel: Arc<AtomicBool>,     // posé par l'UI (bouton Annuler)
        progress: Arc<AtomicU32>,    // permilles 0..=1000, lu par la barre d'état
    },
}

pub enum BatchResult {
    /* … */
    Compressed { out: Option<PathBuf>, report: CompressReport, verification: Option<Verification> },
    Cancelled,
}
```

Branchement dans `run_batch` (`worker.rs`, à côté de `BatchOp::Optimize`, ~ligne 603) :

```rust
BatchOp::Compress { out, options, cancel, progress } => {
    let doc = doc.ok_or(EngineError::Other("no document".into()))?;
    if sunipdf_core::compress::same_file(&out, &doc.info().path) {
        return Err(EngineError::Other("output must differ from the source".into()));
    }
    let before = doc.snapshot()?;
    let mut tick = |done: usize, total: usize| {
        progress.store((done * 1000 / total.max(1)) as u32, Ordering::Relaxed);
        if cancel.load(Ordering::Relaxed) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    match sunipdf_core::compress::compress_pdf(&before, &options, &mut tick)? {
        CompressOutcome::Cancelled => Ok(BatchResult::Cancelled),
        CompressOutcome::Done { bytes: None, report } =>
            Ok(BatchResult::Compressed { out: None, report, verification: None }),   // « Aucun gain » : rien d'écrit
        CompressOutcome::Done { bytes: Some(bytes), report } => {
            let verification = engine.verify_compressed(doc, &bytes)?;               // Err → BatchResult::Failed
            sunipdf_core::write_atomic(&out, &bytes)?;                               // écrit seulement après vérification
            Ok(BatchResult::Compressed { out: Some(out), report, verification: Some(verification) })
        }
    }
}
```

(`return Err(..)` dans la fermeture `(|| match op { … })()` de `run_batch` est valide : elle renvoie `Result<BatchResult, _>`.)
L'erreur de `verify_compressed` ressort par `BatchResult::Failed(EngineError::VerificationFailed)` (déjà géré par
`on_batch_done`). Document ou onglet fermé pendant la tâche : sans effet, la tâche travaille sur ses propres octets.

---

## 5. Interface

### 5.1 Commande (registre, `crates/sunipdf-core/src/command.rs`)

- Variante `Compress` dans `enum Command`, après `Optimize` (ligne 50).
- Entrée de `REGISTRY` après celle d'`Optimize` (~ligne 446) :
  `cmd!(Compress, "convert.compress", "menu.file.compress", None, true, Convert),`
  Id `convert.compress`, libellé `menu.file.compress`, **pas de raccourci** (aucun libre évident ; la palette Ctrl+K suffit),
  `needs_document = true`, menu **Convertir**. Les tests existants restent valides (`Menu::File` compte 7, pas `Convert`).
  Le registre compte aujourd'hui 63 entrées `cmd!` (comptées dans `command.rs`) et passe à 64 ; `PROJECT_STATUS.md` cite encore
  « 57 commandes » à la ligne SUNI-010 (chiffre historique) : ne pas recopier d'ancien nombre.
  Le menu Convertir et la palette lisent le registre (`Command::in_menu`, `app.rs:1027`) : l'entrée apparaît sans autre code.

### 5.2 Fichiers et fonctions à toucher

| Fichier | Modification |
|---|---|
| `apps/sunipdf-desktop/src/shell.rs` | `ToolTab::All` : `Command::Compress` après `Command::Optimize` (ligne 71) ; `ToolTab::Convert` : idem (ligne 101) ; `command_icon` : `Command::Compress => (glyph::OPTIMIZE, "⇊")` (même glyphe qu'`Optimize`, repli texte distinct, voir questions ouvertes) ; `command_color` inchangé (`Menu::Convert` → `CAT_CONVERT`) ; `status_bar` (ligne 1122) : bloc progression + Annuler (5.4) |
| `apps/sunipdf-desktop/src/app.rs` | `execute` : arme `Command::Compress` (5.3) ; `handle_input` (ligne 576) : ajouter `\|\| self.tools.compress.is_open()` à la garde des modales (ligne ~594) ; `update` : `self.compress_dialog(&ctx);` après `self.stamp_dialog(&ctx);` (ligne 1877) ; `error_key` (ligne 1924) : `EngineError::VerificationFailed => "error.verification_failed"` ; test `every_error_key_exists_in_both_catalogs` (ligne 1943) : ajouter la variante au tableau |
| `apps/sunipdf-desktop/src/tools.rs` | état `CompressUi`, `compress_dialog` (options puis résultat), `compress_submit`, arme `on_batch_done` pour `Compressed` et `Cancelled`, remise à zéro de `compress.running` en tête de `on_batch_done`, `human_size` internationalisé (5.5) |
| `apps/sunipdf-desktop/src/worker.rs` | section 4 |
| `apps/sunipdf-desktop/src/docinfo.rs` | `human_size` (ligne 23) remplacé par l'appel à la version partagée de `tools.rs` |
| `crates/sunipdf-core/src/error.rs` | variante `VerificationFailed` + `code()` |
| `crates/sunipdf-core/src/lib.rs` | `pub mod compress; pub mod verify;` + ré-exports |
| `apps/sunipdf-cli/src/main.rs` | sous-commande `Compress` (6) |
| `assets/i18n/fr-FR.json`, `en-US.json` | clés de la section 7 |

### 5.3 Interaction

`execute(ctx, Command::Compress)` : si `self.tools.compress.running` → ne rien faire ; sinon
`self.docinfo.properties = None; self.worker.send(Request::Properties);` (pour connaître `encrypted`, même patron que
`Command::Properties`) puis `self.tools.compress.stage = CompressStage::Options`.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressChoice { Screen, #[default] Ebook, Print, Custom }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressStage { #[default] Closed, Options, Result }

pub struct CompressSummary { pub out: Option<PathBuf>, pub report: CompressReport, pub verification: Option<Verification> }

pub struct CompressUi {
    pub stage: CompressStage,
    pub choice: CompressChoice,
    pub dpi: f32,                     // valeur du mode Personnalisé (défaut 150.0)
    pub quality: u8,                  // idem (défaut 75)
    pub running: bool,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<AtomicU32>,
    pub summary: Option<CompressSummary>,
}
impl Default for CompressUi { /* dpi 150.0, quality 75, reste par défaut */ }
impl CompressUi { pub fn is_open(&self) -> bool { self.stage != CompressStage::Closed } }
/// Pur et testable sans egui.
pub fn choice_options(choice: CompressChoice, dpi: f32, quality: u8) -> CompressOptions;
```

`ToolsState` (déjà `#[derive(Default)]`) reçoit `pub compress: CompressUi`.

**Boîte d'options** (`dialog::show`, id `"compress"`, largeur 520, icône `glyph::OPTIMIZE` / repli `"⇊"`, sous-titre
`compress.subtitle`) :
- corps : si `self.docinfo.properties.as_ref().map(|p| p.encrypted) == Some(true)` → bloc d'avertissement
  (`theme::ERROR`, texte `compress.encrypted_notice`) ; 4 `ui.radio_value(&mut c.choice, …)` précédés/suivis de la ligne
  d'explication en `text_muted` 12 px (clés `compress.preset.*` et `.desc`) ; si `Custom` : `DragValue` résolution
  (`36.0..=600.0`) et qualité (`10..=95`) ; note `compress.note_kept` en muted ;
- pied : `dialog::primary(ui, tr(lang, "compress.ok"))` enveloppé dans `ui.add_enabled_ui(!encrypted, …)` (le bouton est désactivé,
  `InnerResponse::inner` donne le clic) ou Entrée ; `dialog::secondary(ui, tr(lang, "dialog.password.cancel"))` ou Échap ferme.
  Pas de bouton factice : « Compresser… » ouvre réellement la boîte d'enregistrement.

**Soumission** (`compress_submit`) : copier `doc.info.path` et libérer l'emprunt de `self.document` ; `let Some(out) =
self.save_dialog("compress.save", &self.suggest_output("-compresse")) else { return }` (`tools.rs:105, 90`) ; si
`same_file(&out, &source)` → `self.error = Some("error.compress_same_file"); return;` ; sinon recréer
`cancel = Arc::new(AtomicBool::new(false))` et `progress = Arc::new(AtomicU32::new(0))`, `stage = Closed`, `running = true`,
envoyer `Request::Batch(BatchOp::Compress { out, options: choice_options(…), cancel, progress })` (envoi direct :
`run_batch` pose `tools.running` avec une clé statique, ici la barre d'état affiche un texte dynamique).

**Barre d'état** (`shell.rs::status_bar`, à côté du bloc `tools.running`, ligne ~1167) : si `compress.running` →
spinner, `tr_args(lang, "compress.running", &[("percent", &(progress.load(Relaxed)/10).to_string())])`, petit bouton
`compress.cancel` qui fait `self.tools.compress.cancel.store(true, Relaxed)` (`&self` suffit, c'est un atomique) et
`ui.ctx().request_repaint_after(Duration::from_millis(100))` pour rafraîchir le pourcentage.

**Fin de tâche** (`on_batch_done`, qui commence déjà par `self.tools.running = None;`, ajouter
`self.tools.compress.running = false;`) :
- `Compressed { out, report, verification }` → `summary = Some(..)`, `stage = Result` ;
- `Cancelled` → `status_msg = Some(tr(lang, "compress.cancelled").into())` ;
- `Failed(e)` → arme existante (`error_key`) ; `VerificationFailed` → `error.verification_failed`.

**Boîte de résultat** (`dialog::show`, id `"compress-result"`) : titre `compress.result.title` (ou
`compress.result.title_nogain` si `out` est `None`) ; lignes : `compress.result.sizes` (`{before} → {after} ({percent} % de
gain)`) ou `compress.result.nogain` ; `compress.result.images` (`{done}` sur `{total}`) ; `compress.result.masks` si
`opaque_masks_dropped > 0` ; `compress.result.kept` suivi d'une ligne par raison `compress.skip.<code>` avec `{count}` ;
`compress.result.verified` (`{pages}`, `{diff}` arrondi à 0,1) ; `compress.result.file`. Pied : si `out` est `Some` →
`dialog::primary(…"compress.open")` (→ `self.open_path(out, None)`, nouvel onglet) + `dialog::secondary(…"compress.close")` ;
sinon `dialog::primary(…"compress.close")`. Échap ferme.

### 5.4 Onglets et annulation (undo)

- **Onglets** : `ToolsState` est global (comme `stamp_open`, `split_open`). La boîte d'options est modale (`egui::Modal`),
  donc aucun changement d'onglet pendant son affichage. Pendant l'exécution, l'utilisateur peut changer d'onglet : la tâche
  travaille sur les octets de l'instantané pris au lancement, et `Request::Suspend` / `Resume` attendent derrière elle dans
  la file du worker (le message « changement d'onglet » `tabs.switching` s'affiche jusqu'à la fin ; limite documentée).
  Le résultat s'affiche quel que soit l'onglet actif. Un deuxième lancement est refusé tant que `running`.
- **Undo** : aucune entrée (le document ouvert n'est pas modifié, un nouveau fichier est produit) — comme `Optimize`.
- **Annulation de la tâche** : cooperative, entre deux images (granularité = une image ; mesurer le temps maximal par image
  sur un scan 600 ppp, étape 7). Résultat : rien n'est écrit.

### 5.5 `human_size` et unités

Les deux copies actuelles (`tools.rs:77`, `docinfo.rs:23`) écrivent des « Ko / Mo » en dur, y compris dans l'interface
anglaise (contraire à la règle « aucune chaîne UI littérale »). Les remplacer par une seule
`pub fn human_size(lang: Language, bytes: u64) -> String` (dans `tools.rs`) qui utilise `tr_args` avec `size.b`, `size.kb`,
`size.mb` et `{value}`, et l'utiliser pour la boîte de résultat, `tools.done.optimize` et la boîte Propriétés.

### 5.6 Impact de la variante d'erreur

`EngineError::VerificationFailed` touche : `error.rs` (variante + `code()`), `app.rs::error_key` et son test (tableau
d'erreurs, ligne ~1945), les deux catalogues (`error.verification_failed`), la CLI (code JSON `verification_failed`, code de
sortie par défaut 1, `main.rs` fin de fichier). Aucun autre `match` exhaustif sur `EngineError` n'existe dans le dépôt
(à confirmer par `grep -rn "EngineError::Other(_) =>"` avant de compiler).

### 5.7 Décision : tâche de fichiers (nouveau fichier), pas `EditOp` en place

Variante rejetée : `EditOp::Bytes(BytesOp::Compress(..))` appliquée au document ouvert sous instantané (annulable). Raisons :
(a) un `EditOp` ne renvoie que `Response::DocumentChanged` : le rapport avant/après et la progression n'ont pas de canal ;
(b) une version avec perte enregistrée par Ctrl+S écraserait l'original de façon définitive ; PDFCraft et les conventions
usuelles produisent un nouveau fichier ; (c) `BatchOp::Optimize` est déjà le précédent direct. Le cas « remplacer le document
ouvert par sa version compressée, annulable » reste possible plus tard (le bouton « Ouvrir le fichier compressé » le couvre
déjà dans un nouvel onglet).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

```rust
/// Lossy size reduction: downsample and re-encode images as JPEG. Writes the output only if it is smaller.
Compress {
    file: PathBuf,
    #[arg(long)] out: PathBuf,
    /// screen (96 dpi, q60) | ebook (150 dpi, q75) | print (300 dpi, q88)
    #[arg(long, default_value = "ebook")] preset: String,
    /// Override the preset's target resolution (36..=600).
    #[arg(long)] dpi: Option<f32>,
    /// Override the preset's JPEG quality (10..=95).
    #[arg(long)] quality: Option<u8>,
    /// Skip the PDFium reopen/render check (not recommended).
    #[arg(long)] no_verify: bool,
    /// Also run `qpdf --check` on the output when qpdf is available (see scripts/fetch-qpdf.ps1).
    #[arg(long)] qpdf_check: bool,
}
```

Flux : `CompressPreset::from_id(&preset).ok_or_else(|| EngineError::Other(format!("unknown preset: {preset}")))` ; surcharges
`--dpi`/`--quality` ; refuser `out == file` (`same_file`, `EngineError::Other`) ; `let doc = engine.open(&file, None)?` (valide le
fichier ; document protégé → `PasswordRequired`, code de sortie 3, cohérent avec les autres commandes) ;
`let bytes = std::fs::read(&file)?` ; `compress_pdf(&bytes, &opts, &mut |_, _| ControlFlow::Continue(()))?` ; si `Some(b)` et
non `--no-verify` → `engine.verify_compressed(&doc, &b)?` ; `write_atomic` ; si `--qpdf-check` → `qpdf_check(&out)`.
Erreurs : JSON `{"error": code, "message": …}` sur stderr (`main`, fin de fichier), codes 3/4/5/1 inchangés ; chiffré →
`security_restricted` (code 1) ; vérification échouée → `verification_failed` (code 1).

Sortie JSON (succès) :

```json
{
  "out": "doc-compressed.pdf", "written": true,
  "before": 12345678, "after": 3456789, "saved_percent": 72.0,
  "preset": "ebook", "target_dpi": 150.0, "jpeg_quality": 75,
  "images": { "found": 14, "recompressed": 11, "kept": { "transparency": 2, "small": 1 } },
  "opaque_masks_dropped": 3,
  "image_bytes": { "before": 11000000, "after": 2100000 },
  "verification": { "pages": 12, "sampled": 3, "max_mean_abs_diff": 2.4 },
  "qpdf": { "status": "ok" }
}
```

(valeurs d'illustration du format, **pas des mesures**.) Sans gain : `"written": false, "reason": "no_gain"`, `before`, `after`
(taille du candidat), aucun fichier créé, code de sortie 0. `"verification"` est absent avec `--no-verify`, `"qpdf"` absent sans
`--qpdf-check` (`status` ∈ `ok | warnings | errors | unavailable`, avec `"detail"` pour les deux du milieu).

---

## 7. i18n — clés à ajouter dans `assets/i18n/fr-FR.json` **et** `en-US.json`

Propriétaire des catalogues : agent ui-ux (D013) ; clés proposées sous les préfixes `compress.*`, `size.*`, `menu.file.*`,
`error.*`. Les deux fichiers ont aujourd'hui 382 clés ; vérifier la parité avec `.\scripts\check-i18n.ps1`.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.compress` | Compresser le PDF (avec perte)… | Compress PDF (lossy)… |
| `compress.title` | Compresser le PDF | Compress PDF |
| `compress.subtitle` | Réduit la taille en rééchantillonnant et en recompressant les images. Un nouveau fichier est créé, l'original n'est pas modifié. | Shrinks the file by downsampling and recompressing images. A new file is created; the original is left untouched. |
| `compress.preset.screen` | Écran | Screen |
| `compress.preset.screen.desc` | 96 ppp, JPEG qualité 60 : fichier le plus petit, lecture à l'écran. | 96 dpi, JPEG quality 60: smallest file, for on-screen reading. |
| `compress.preset.ebook` | Ebook (équilibré) | Ebook (balanced) |
| `compress.preset.ebook.desc` | 150 ppp, JPEG qualité 75 : bon compromis entre taille et netteté. | 150 dpi, JPEG quality 75: good balance between size and sharpness. |
| `compress.preset.print` | Impression | Print |
| `compress.preset.print.desc` | 300 ppp, JPEG qualité 88 : proche de l'original, gain plus modeste. | 300 dpi, JPEG quality 88: close to the original, smaller saving. |
| `compress.preset.custom` | Personnalisé | Custom |
| `compress.dpi` | Résolution cible (ppp) | Target resolution (dpi) |
| `compress.quality` | Qualité JPEG | JPEG quality |
| `compress.note_kept` | Restent inchangées : les images avec transparence ou masque, en noir et blanc 1 bit, en CMJN, déjà petites ou déjà à la résolution cible. | Left untouched: images with transparency or masks, 1-bit black and white, CMYK, already small or already at the target resolution. |
| `compress.encrypted_notice` | Ce document est protégé par un mot de passe. Retirez d'abord la protection (menu Sécurité), puis compressez. | This document is password-protected. Remove the protection first (Security menu), then compress. |
| `compress.ok` | Compresser… | Compress… |
| `compress.save` | Enregistrer le PDF compressé | Save compressed PDF |
| `compress.running` | Compression… {percent} % | Compressing… {percent}% |
| `compress.cancel` | Annuler la compression | Cancel compression |
| `compress.cancelled` | Compression annulée : aucun fichier écrit. | Compression cancelled: no file written. |
| `compress.close` | Fermer | Close |
| `compress.open` | Ouvrir le fichier compressé | Open compressed file |
| `compress.result.title` | Compression terminée | Compression finished |
| `compress.result.title_nogain` | Aucun gain | No saving |
| `compress.result.sizes` | {before} → {after} ({percent} % de gain) | {before} → {after} ({percent}% saved) |
| `compress.result.nogain` | Le résultat ({after}) n'est pas plus petit que l'original ({before}) : aucun fichier n'a été écrit. | The result ({after}) is not smaller than the original ({before}): no file was written. |
| `compress.result.file` | Fichier : {name} | File: {name} |
| `compress.result.images` | {done} image(s) recompressée(s) sur {total}. | {done} of {total} image(s) recompressed. |
| `compress.result.masks` | {count} masque(s) de transparence entièrement opaque(s) retiré(s). | {count} fully opaque transparency mask(s) removed. |
| `compress.result.kept` | Images conservées telles quelles : | Images kept as they were: |
| `compress.result.verified` | Vérifié : {pages} page(s) rouvertes, écart visuel moyen {diff}/255. | Verified: {pages} page(s) reopened, mean visual difference {diff}/255. |
| `compress.skip.unmeasured` | {count} d'usage indéterminé (annotation, motif…) | {count} of unknown use (annotation, pattern…) |
| `compress.skip.small` | {count} déjà petite(s) | {count} already small |
| `compress.skip.stencil` | {count} masque(s) d'image | {count} image mask(s) |
| `compress.skip.transparency` | {count} avec transparence | {count} with transparency |
| `compress.skip.colorspace` | {count} en CMJN ou espace colorimétrique non géré | {count} in CMYK or an unsupported colour space |
| `compress.skip.bitdepth` | {count} en noir et blanc 1 bit ou profondeur non gérée | {count} 1-bit black and white or unsupported bit depth |
| `compress.skip.filter` | {count} dans un format non géré (JPEG 2000, JBIG2, CCITT…) | {count} in an unsupported format (JPEG 2000, JBIG2, CCITT…) |
| `compress.skip.too_large` | {count} trop grande(s) | {count} too large |
| `compress.skip.corrupt` | {count} illisible(s) | {count} unreadable |
| `compress.skip.graphic` | {count} graphique(s) à aplats, gardés sans perte | {count} flat-colour graphic(s), kept lossless |
| `compress.skip.resolution` | {count} déjà à la résolution cible | {count} already at the target resolution |
| `compress.skip.no_gain` | {count} sans gain | {count} with no saving |
| `error.verification_failed` | Le fichier compressé n'a pas passé la vérification (pages ou rendu différents). Il n'a pas été écrit. | The compressed file failed verification (pages or rendering differ). It was not written. |
| `error.compress_same_file` | Choisissez un autre nom : la compression ne remplace jamais le fichier d'origine. | Choose a different name: compression never overwrites the original file. |
| `size.b` | {value} o | {value} B |
| `size.kb` | {value} Ko | {value} KB |
| `size.mb` | {value} Mo | {value} MB |

47 clés au total (comptées dans le tableau ci-dessus). `home.card.convert.desc` peut aussi gagner « · Compresser » / « · Compress » (modification d'une clé existante,
facultative). Contraintes : les noms de fichiers (`{name}`) ne sont jamais traduits ; `tr_args` remplace `{percent}`, `{before}`,
`{after}`, `{done}`, `{total}`, `{count}`, `{pages}`, `{diff}`, `{name}`, `{value}` (vérifier que `sunipdf-i18n` ne traite pas les
accolades autrement, `crates/sunipdf-i18n`).

---

## 8. Tests

### 8.1 Générateur de fixtures (aucun binaire versionné)

Les PDF de test sont **construits en mémoire** dans le code de test avec `lopdf` (dépendance normale du crate, utilisable par
les tests d'intégration) et `image` (déjà en `dev-dependencies`), ce qui évite un fichier binaire et rend les images
déterministes. `scripts/make_fixtures.py` reste inchangé (Python sans bibliothèque d'image ne produit pas de JPEG).

```rust
// dans compress.rs (#[cfg(test)] mod tests) ET dupliqué minimalement dans tests/engine.rs (mod imgpdf)
fn photo_rgb(w: u32, h: u32) -> Vec<u8>;                 // dégradé r=x, g=y, b=x+y + bruit xorshift ±6, déterministe
fn flate_rgb(w: u32, h: u32, rgb: &[u8]) -> lopdf::Stream; // /Subtype /Image /ColorSpace /DeviceRGB /BitsPerComponent 8, puis .compress()
fn jpeg_rgb(w: u32, h: u32, rgb: &[u8], q: u8) -> lopdf::Stream; // JpegEncoder::new_with_quality + /Filter /DCTDecode
fn page_pdf(images: Vec<(lopdf::Stream, (f32, f32, f32, f32))>) -> Vec<u8>; // page 612×792 ; "q w 0 0 h x y cm /ImN Do Q"
```
Construction minimale d'un PDF lopdf : `Document::with_version("1.5")`, `new_object_id()` pour `Pages`, `add_object` pour image, contenu
(`Stream::new(dictionary!{}, …)`), page (`dictionary!{"Type"=>"Page","Parent"=>pages_id,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content_id,"Resources"=>dictionary!{"XObject"=>dictionary!{"Im1"=>img_id}}}`),
`doc.objects.insert(pages_id, …)`, catalogue, `doc.trailer.set("Root", catalog_id)`, `save_to(&mut Vec)`
(`LO/creator.rs:8-21`, `LO/object.rs:677-693` : `dictionary!` accepte la virgule finale).

### 8.2 Tests unitaires sans PDFium (`#[cfg(test)]` dans `compress.rs`, rapides)

| Test | Assertions |
|---|---|
| `mul_composes_like_pdf_cm`, `displayed_size_with_rotation_and_shear` | `mul` associatif sur des cas connus ; `hypot(a,b)` inchangé par rotation 90° |
| `target_dims_never_upscale_and_respects_threshold` | 1800 px sur 5 in = 360 ppp, cible 150 → 750 ; 300 ppp à cible 150 (seuil 187,5) → inchangé ; plancher 8 px |
| `skip_reason_codes_are_unique_and_ordered` | 12 codes distincts ; ordre de priorité = ordre des variantes |
| `components_of_colorspaces` | Gray/RGB/CalRGB/ICCBased N=1,3 → 1/3 ; CMYK, ICCBased N=4, Indexed, Separation → `None` |
| `looks_like_photo_vs_flat` | dégradé bruité → photo ; 4 aplats → graphique (seuils à ajuster, étape 4) |
| `options_validate` | dpi 10 ou 9000, qualité 0 ou 100 → `Err` ; les 3 préréglages valides |
| `photo_flate_becomes_dct_with_expected_dims` | image Flate RGB 1800×1200 posée 360×240 pt, préréglage Ebook : 1 recompressée ; relecture lopdf : `/Filter /DCTDecode`, `/Width 750`, `/Height 500`, `/ColorSpace /DeviceRGB` conservé, `/BitsPerComponent 8` ; `after < before / 10` ; le flux décodé par `image` fait 750×500 |
| `dct_is_downsampled_and_low_res_dct_is_kept` | JPEG 3000×2000 sur 6×4 in (500 ppp), Écran → 576×384 ; JPEG 600×400 sur 6×4 in (100 ppp), Ebook → octets du flux **identiques**, raison `resolution` (`min_stream_bytes` abaissé à 1 024 dans le test) |
| `dpi_is_measured_through_forms_and_rotation` | image 1000×1000 dans un formulaire de `Matrix [0.5 0 0 0.5 0 0]` dont le contenu fait `288 0 0 288 0 0 cm /Im1 Do` et appelé avec une rotation 90° : 144 pt = 2 in = 500 ppp ; Ebook → 300×300 ; image utilisée deux fois (petite puis grande) → dimensions pour la **grande** |
| `unmeasured_image_is_untouched` | image présente dans les ressources mais jamais appelée par `Do` → `Unmeasured`, flux identique |
| `transparency_is_never_touched` | image RGB + `/SMask` semi-transparent ; image avec `/Mask [0 0]` (clé de couleur) ; image avec `/Mask` flux → toutes `Transparency`, octets du flux **et** du SMask identiques, `/SMask` toujours référencé |
| `opaque_smask_is_dropped` | image Flate + SMask Flate DeviceGray tout à 255 : recompressée, `/SMask` absent, flux SMask absent du fichier (élagué) ; avec un seul pixel à 254 → `Transparency` |
| `skip_precedence_and_small_cmyk_1bit_indexed_mask` | 6 images : petite (<16 Kio), 1 bit gris 2000×2000, CMJN Flate 4 composantes, Indexed 8 bits, `/ImageMask true`, graphique à 4 couleurs : raisons `small`, `bitdepth`, `colorspace`, `colorspace`, `stencil`, `graphic` ; toutes les octets de flux inchangés |
| `corrupt_flate_and_bad_decodeparms_are_skipped` | flux Flate tronqué (longueur < w·h·3) → `Corrupt` ; `/DecodeParms` indirect → `Filter` ; image JPEG dont l'en-tête dépasse `/Width` → `Corrupt` |
| `not_larger_rule` | bruit blanc 300×300 qualité 95 (JPEG plus gros que Flate) → `NotSmaller` ; PDF sans image → `bytes` est `None` ou strictement plus petit (jamais ≥ entrée) |
| `encrypted_is_refused` | sortie de `security::protect` → `Err(SecurityRestricted)` |
| `cancel_stops_before_any_change` | fermeture qui renvoie `Break` au premier appel → `Cancelled` ; progression appelée avec `(0, total)` |
| `progress_reaches_total` | appels `(i, total)` monotones |

### 8.3 Tests d'intégration PDFium (`crates/sunipdf-core/tests/engine.rs`, moteur partagé sérialisé)

Ajouter en bas du fichier un module `imgpdf` (générateurs ci-dessus) et ces tests :

| Test | Assertions |
|---|---|
| `compress_photo_renders_like_original` | PDF de la fixture « photo Flate 1800×1200 sur 360×240 pt », Ebook : le résultat s'ouvre dans PDFium ; `engine.verify_compressed` réussit ; rendu des deux fichiers à `scale 1.0` : écart moyen absolu < seuil mesuré ; pixel central à ±12 par canal de l'original ; `report.after < report.before / 10` |
| `compress_dct_downsample_renders_like_original` | idem avec le JPEG 3000×2000, Écran |
| `compress_transparency_renders_identically` | image RGB + SMask semi-transparent (moitié gauche opaque, moitié droite à 50 %) : image ignorée, rendu **identique** (écart 0 hors recompression sans perte) avant/après |
| `compress_images_to_pdf_output_drops_opaque_mask` | `engine.images_to_pdf` sur un PNG photographique 1200×800 à 150 ppp (donc Flate + SMask opaque, voir 2.1), puis `compress_pdf` : image recompressée, `/SMask` retiré, rendu équivalent ; sans cette étape l'image serait ignorée à tort |
| `verify_rejects_bad_candidates` | candidat de nombre de pages différent (`two-pages.pdf` contre un document d'une page) → `Err(VerificationFailed)` ; candidat dont l'image a été remplacée par un JPEG noir de mêmes dimensions → `Err(VerificationFailed)` (sert à **calibrer** le seuil : consigner l'écart mesuré dans chaque cas) |
| `compress_output_passes_qpdf_when_available` | si `find_qpdf()` : `qpdf_check` renvoie `Ok` (ou `Warnings` consignés) sur les sorties des tests précédents ; sinon le test affiche « qpdf indisponible : vérification ignorée » et passe, **sauf si `SUNIPDF_REQUIRE_QPDF=1`** (échec). Le compte rendu de l'implémentation doit dire si qpdf a réellement tourné |
| `compress_file_is_atomic_and_never_overwrites_source` | via `write_atomic` : aucun `.tmp` résiduel ; `same_file(src, src)` vrai, `same_file(src, autre)` faux |

Commandes de validation (à exécuter par l'implémenteur, résultats réels à rapporter) :
`cargo test -p sunipdf-core -j 2` ; `cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2` ; `.\scripts\check-i18n.ps1` ;
`cargo clippy --workspace --all-targets -j 2 -- -D warnings` ; `cargo fmt --all -- --check` ;
`cargo test --workspace -j 2`.

### 8.4 Scénario manuel avec `scripts/dev-screenshot.ps1`

Préparation (jetable, hors dépôt ; Python sans dépendance, `python -I`) : écrire un PNG photographique 2400×1600 (dégradé +
bruit, `zlib` + `struct`, chunks IHDR/IDAT/IEND), puis
`cargo run -p sunipdf-cli -- images2pdf demo.png --dpi 300 --out demo.pdf` (donne exactement le motif Flate + SMask opaque).
Mesure de référence en ligne de commande : `cargo run -p sunipdf-cli -- compress demo.pdf --preset ebook --out demo-c.pdf --qpdf-check`
(noter la sortie JSON réelle) puis `render` des deux fichiers et comparaison visuelle des PNG.

1. Boîte d'options : `.\scripts\dev-screenshot.ps1 -Out compress-options.png -Pdf demo.pdf -Commands 'convert.compress'`
   → vérifier : titre, sous-titre, 4 radios avec descriptions, note, pied à deux boutons, thème clair puis sombre.
2. Mode Personnalisé : relever sur la capture 1 les coordonnées (points logiques, relatives à la fenêtre) du radio
   « Personnalisé », puis `-Drag 'x,y,x,y'` (glissement de longueur nulle = clic) ;
   `.\scripts\dev-screenshot.ps1 -Out compress-custom.png -Pdf demo.pdf -Commands 'convert.compress' -Drag 'x,y,x,y'` → champs
   résolution et qualité visibles.
3. Résultat : la boîte « Enregistrer sous » native ne se pilote pas par `SendKeys`. Ajouter un crochet de développement
   `#[cfg(debug_assertions)]` dans `save_dialog` (`tools.rs:105`) : si `SUNIPDF_AUTO_OUT_DIR` est défini, renvoyer
   `<dir>/<nom suggéré>` sans ouvrir `rfd` (même esprit que `SUNIPDF_STARTUP_COMMANDS`, absent des builds release). Puis
   `$env:SUNIPDF_AUTO_OUT_DIR="$env:TEMP\sunipdf-shots"; .\scripts\dev-screenshot.ps1 -Out compress-result.png -Pdf demo.pdf -Commands 'convert.compress' -KeysAfter '{ENTER}'`
   → vérifier la boîte de résultat (tailles, images, vérification, bouton « Ouvrir »). Cas « Aucun gain » : relancer sur `two-pages.pdf`.
4. Annulation : sur un PDF de plusieurs dizaines d'images, lancer puis cliquer « Annuler » dans la barre d'état ; vérifier le
   message `compress.cancelled` et l'absence de fichier de sortie.

Les captures sont à regarder par l'implémenteur (les décrire, ne pas les supposer).

---

## 9. Nouvelles dépendances

**Aucune.** `lopdf` 0.45 (MIT) et `image` 0.25.10 (MIT OR Apache-2.0, fonctionnalité `jpeg` déjà activée au niveau du workspace)
sont déjà des dépendances de `sunipdf-core` ; le décodage JPEG passe par zune-jpeg 0.5.15 et l'encodage par l'encodeur
intégré à `image`, tous deux déjà dans `Cargo.lock`. `std::ops::ControlFlow`, `std::process::Command` et les atomiques viennent de la
bibliothèque standard. Aucun appel réseau (revue des dépendances : rien n'est ajouté). **qpdf** (Apache-2.0) est un exécutable de
développement obtenu par script, jamais lié ni livré.

---

## 10. Risques, pièges, ordre d'implémentation, estimation

### 10.1 Risques et pièges connus

1. **Le thread moteur est occupé** pendant la compression (rendus et changements d'onglet attendent). Atténué par la progression
   et l'annulation ; amélioration possible : déplacer `compress_pdf` (pur Rust, sans PDFium) vers un `std::thread` avec un
   `Sender<Request>` de retour, puis ne garder sur le thread moteur que la vérification et l'écriture. Noter dans KNOWN_ISSUES.
2. **Temps et mémoire non mesurés** : décodage + `resize` Lanczos3 + encodage JPEG d'une image 300 ppp A4 (~8,7 Mpx) en RGB
   (`rayon` n'est pas activé dans le workspace, tout est mono-thread). À mesurer à l'étape 7 sur un scan réel ; pistes si trop
   lent : `FilterType::Triangle` au-delà de 50 Mpx, ou images traitées en parallèle par `std::thread::scope` (travaux
   indépendants, tampons possédés ; attention à la mémoire du poste, 14 Go, `-j 2`).
3. **Décodeur JPEG** : JPEG Adobe avec transformation 0 (RGB direct), progressifs, ou en-têtes mensongers. Filets : limites de
   dimensions strictes, contrôle de cohérence couleur, `/ColorTransform`, vérification visuelle finale. À tester avec de vrais
   JPEG (aucun corpus disponible ici : proposer 3 PDF libres de droits dans le futur corpus SUNI-016).
4. **Heuristique photo / graphique** non calibrée sur des documents réels : en cas d'erreur, le pire cas est un schéma recodé
   en JPEG (garde « jamais plus gros » mais artefacts visibles) ou une photo laissée telle quelle (gain manqué). Seuils exposés en
   constantes nommées, tests dédiés.
5. **Sous-échantillonnage 4:2:2 imposé** par l'encodeur d'`image` (`encoder.rs:468-478`) : franges colorées sur du texte
   coloré fin ; documenté, pas de réglage disponible.
6. **Réécriture lopdf** : linéarisation et signatures numériques perdues, flux d'objets éclatés. Un PDF riche en petits objets
   peut grossir : la règle « jamais plus gros » le détecte. Piste : essayer `save_with_options`/`save_modern`
   (`LO/writer.rs:30, 57`) et garder le plus petit, après mesure et passage qpdf (non fait). Les PDF/A perdent leur conformité
   revendiquée (à dire dans le guide).
7. **Images aussi dessinées depuis une annotation ou un motif** : dimensionnées d'après leur usage de page seulement ; peu de
   risque, mais possible flou d'un tampon d'annotation partageant l'image.
8. **Seuil de vérification** (20,0) arbitraire tant qu'il n'est pas mesuré : voir 3.4.
9. **`optimize` existant retire le chiffrement en silence** (`convert.rs:94-111`, `security.rs:411-427`) : défaut voisin,
   hors tranche, à signaler dans KNOWN_ISSUES (la compression, elle, refuse).
10. **`images_to_pdf` produit des PDF gonflés** (Flate + SMask opaque, 2.1) : amélioration voisine hors tranche (transmettre le
    JPEG d'origine avec `new_from_jpeg_reader`) ; la compression en récupère une partie.
11. **Nouvelle variante `EngineError`** : tout `match` exhaustif doit être mis à jour (5.6).
12. **`Stream::compress()` ne recompresse que les flux sans `/Filter`** (`LO/object.rs:857-870`) : ne touche donc pas à nos flux DCT (voulu).
13. **Compatibilité PDFium des JPEG produits** : relue par PDFium dans les tests (rendu), indépendante par qpdf (structure
    seulement) ; pas de test avec d'autres lecteurs (Acrobat, navigateurs) sur ce poste.
14. **CI GitHub non vérifiable ici** (KNOWN_ISSUES A10) : le pas qpdf du workflow est écrit sans pouvoir être exécuté.

### 10.2 Ordre d'implémentation (étapes compilables, chacune se termine par ses tests)

1. **Types et fonctions pures** : `compress.rs` (options, préréglages, `SkipReason`, `Report`, `mul`, `target_dims`, `components`,
   `looks_like_photo`, `same_file`), `lib.rs`, tests unitaires purs. `cargo test -p sunipdf-core --lib -j 2`.
2. **Mesure des usages** (parcours `q/Q/cm/Do`, formulaires, `UserUnit`, ressources héritées) + tests `dpi_is_measured_*` et
   `unmeasured_*` avec PDF lopdf synthétiques.
3. **Classification + décodage + recodage + application + passe sans perte** : chemin heureux Flate→JPEG et JPEG réduit ;
   tests `photo_flate_*`, `dct_*`, `not_larger_rule`.
4. **Règles de refus** (transparence, SMask opaque, petites, 1 bit, CMJN, indexé, masque, corrompu, chiffré, annulation) +
   **calibrage** de l'heuristique photo et mesure de l'écart de vérification ; consigner les chiffres mesurés.
5. **`verify.rs`** : `Verification`, `Engine::verify_compressed`, variante `EngineError::VerificationFailed` (+ `error_key` et son
   test dans le bureau, clé i18n), `qpdf_check`/`find_qpdf` ; tests d'intégration PDFium (rendu, rejet de candidats, qpdf si présent).
6. **CLI `compress`** + exécution réelle sur `demo.pdf` (sortie JSON réelle à rapporter) ; test unitaire d'analyse d'arguments.
7. **Worker + interface** : registre (`Command::Compress`), `BatchOp`/`BatchResult`, `tools.rs` (état, dialogues, soumission,
   fin de tâche), `shell.rs` (liste d'outils, icône, barre d'état), `app.rs` (exécution, garde des modales, mapping d'erreur),
   catalogues FR/EN, `human_size` internationalisé ; `cargo test -p sunipdf-i18n -p sunipdf-desktop`, `check-i18n.ps1`,
   scénario de captures 8.4, **mesure du temps par image** sur un vrai scan.
8. **`scripts/fetch-qpdf.ps1`**, `.gitignore`, pas CI facultatif, exécution réelle du script et de `compress --qpdf-check`
   (consigner si qpdf a tourné ou non).
9. **Extensions facultatives**, chacune après mesure : réduction seule des graphiques Flate (sans JPEG), traitement parallèle,
   `save_modern`, transparence (rééchantillonnage conjoint image + SMask), estimation du gain avant exécution.

Les étapes 1 à 6 sont indépendantes de l'interface et testables sans écran ; les étapes 7 et 8 peuvent se faire en parallèle
une fois l'API de l'étape 5 figée.

### 10.3 Estimation

**L** : environ 700 à 900 lignes de moteur (`compress.rs` ≈ 550, `verify.rs` ≈ 150), ≈ 350 lignes d'interface, ≈ 80 de CLI,
≈ 600 de tests, 47 clés × 2 langues, un script PowerShell, de la documentation. Les incertitudes principales sont la
calibration (heuristique photo, seuil de vérification) et le temps de traitement réel ; l'estimation d'effort elle-même est un
jugement, pas une mesure.

### 10.4 Questions ouvertes (à trancher par l'orchestrateur ou l'utilisateur)

1. Glyphe dédié pour `Compress` (le glyphe `OPTIMIZE` est partagé ; ajouter une constante Segoe Fluent demande de vérifier le
   point de code dans la police du poste, non fait ici).
2. Un raccourci clavier pour `convert.compress` est-il souhaité ? (aucun proposé.)
3. Conserver le crochet de développement `SUNIPDF_AUTO_OUT_DIR` (debug seulement) ou se contenter de captures 1 et 2 ?
4. Autoriser plus tard un mode « remplacer le document ouvert » annulable (rejeté pour la v1, 5.7).
5. Ajouter `compress` aux guides utilisateur avant l'étape 9 ou seulement après les extensions ?

---

## 11. Documentation à mettre à jour (à la fin, avec les résultats réels uniquement)

- `docs/PROJECT_STATUS.md` : nouvelle ligne SUNI-025 (compression avec perte), commandes exécutées et résultats réels, mesures
  de calibrage (écart de vérification, temps par image), nombre de commandes du registre (64 après ajout), nombre de clés i18n, nombre de tests.
- `docs/NEXT_TASKS.md` : réécrire SUNI-025 — retirer la compression et la partie « objets image → DynamicImage » (la voie
  PDFium est abandonnée, 2.1) ; conserver « vérification qpdf en CI », « fixtures AES produites par qpdf », « test sauvegarde interrompue ».
- `docs/FEATURE_MATRIX.md` : ligne compression de PDFCraft → **Fait (partiel)** avec la liste des exclusions (2.3).
- `docs/ROADMAP.md` : ligne 23 (la mention `compress-lossy.md` devient « fait, extensions en étape 9 ») et ligne 56 (compression).
- `docs/DECISIONS.md` : **D021** « Compression avec perte par réécriture lopdf des XObjects image (pas d'objets image PDFium) ;
  JPEG via `image` ; images à transparence/CMJN/1 bit conservées ; sortie en nouveau fichier ; jamais plus gros ; vérification par
  relecture PDFium » ; **D022** « qpdf = outil de vérification de développement (Apache-2.0), obtenu par `scripts/fetch-qpdf.ps1`
  avec SHA256 épinglé, jamais embarqué ni appelé par l'application ».
- `docs/KNOWN_ISSUES.md` : worker occupé pendant la compression ; limites de périmètre (2.3) ; signatures et PDF/A non préservés ;
  documents chiffrés refusés ; heuristique photo/graphique et seuil de vérification à calibrer sur corpus réel ; `optimize` retire
  le chiffrement en silence ; `images_to_pdf` gonfle les fichiers ; qpdf non exécuté en CI.
- `docs/THIRD_PARTY_NOTICES.md` et `docs/TECH_STACK.md` : qpdf (Apache-2.0, outil de développement non redistribué, version 12.4.2).
- `docs/I18N.md` : espace `compress.*` et `size.*`.
- `docs/USER_GUIDE.fr.md` et `docs/USER_GUIDE.en.md` : section « Compresser un PDF » (préréglages, ce qui n'est pas modifié,
  documents protégés à déprotéger d'abord, signatures invalidées) et correction de la ligne « Optimiser » pour la distinguer.
- `CHANGELOG.md` (section « Non publié ») : « Compression avec perte (préréglages Écran / Ebook / Impression), CLI `compress`, vérification de sortie ».
- `README.md` : commande CLI `compress` si le README liste les commandes ; `CLAUDE.md` : ligne de commandes `fetch-qpdf.ps1` (à valider par le propriétaire du fichier).
- `.gitignore` : `/third_party/qpdf/`.
