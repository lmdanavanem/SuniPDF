# Plan d'implémentation — Annotations avancées (SUNI-021b, slug `annotations-advanced`)

Date : 2026-10-14 · Version cible : 0.4.0 · Équivalent PDFCraft : outil « Add comments » (Ajouter des commentaires)
Statut : plan uniquement (aucun code écrit, aucun `cargo` lancé). Les faits marqués **[Fn]** sont détaillés dans l'annexe A
(citations `fichier:ligne` des sources réelles, ou expériences réalisées sur le `pdfium.dll` 7881 du dépôt).

## 0. Résumé des décisions

1. **Création et édition des nouvelles annotations par `lopdf`** (dictionnaire d'annotation + flux d'apparence `/AP` écrit
   explicitement), pas par `pdfium-render` : l'API de `pdfium-render` 0.9.4 ne sait ni créer un cercle ni une ligne, ne peut pas
   ajouter de trait d'encre, ne donne pas accès à `/DA`, `/BS`, `/IRT`, et son repli de couleur est dangereux **[F1][F3][F4]**.
   PDFium reste le moteur de **rendu**, de **relecture** (liste, liens) et d'**instantané** (undo).
2. **API moteur en « coordonnées de page affichée »** (points, origine en bas à gauche de la zone visible, rotation `/Rotate`
   appliquée). Elle coïncide avec les points PDF pour une page sans rotation ni CropBox décalé, et corrige les pages tournées
   pour les nouveaux outils **[F8]**. Les outils existants (texte, objets, formes) ne sont pas touchés.
3. **Liens** : lecture via `annotations()` + `PdfPageLinkAnnotation::link()` (jamais `page.links().iter()`, qui duplique **[F9]**) ;
   saut interne direct ; URL externe uniquement après boîte de confirmation et liste d'autorisation `http`/`https`/`mailto` **[F14]**.
4. **Documents chiffrés : refus typé** (`EngineError::SecurityRestricted`) pour les écritures lopdf, car `security::load` retire
   `/Encrypt` **[F11]**. Limite assumée et documentée.
5. **Fils de réponses et statuts : réalisés (partiel)** par des annotations `Text` liées par `/IRT` (modèle ISO 32000-1 12.5.6.4),
   sans icône sur la page. Relu et affiché par SuniPDF ; compatibilité Acrobat **non vérifiée** ici.
6. **Aucune nouvelle dépendance.** Effort global **XL**, livrable en deux paquets (§10).

---

## 1. Objectif et comportement visible

Aligner SuniPDF sur « Add comments » de PDFCraft (ligne « Notes, surlignage, souligné, barré, formes, encre, zones de texte » et
« Fils de réponses, statut des commentaires » de `docs/FEATURE_MATRIX.md`) : les marques sont de **vraies annotations PDF**
(`/Subtype /Ink`, `/FreeText`, `/Square`, `/Circle`, `/Line`, `/Link`), pas du contenu de page ; elles restent sélectionnables,
déplaçables, recolorables et supprimables, et s'affichent dans les autres lecteurs grâce à un `/AP` explicite.

| Ce que voit l'utilisateur | Détail |
|---|---|
| Palette flottante (catégorie « Annoter ») | 7 nouveaux outils : **Dessin** (`D`), **Texte libre** (`F`), **Rectangle**, **Ellipse**, **Ligne/Flèche** (annotations), **Lien**, **Sélectionner une annotation** (`A`). Info-bulles et menu Édition via le registre de commandes. |
| Dessin à main levée | Glisser sur la page : trait lissé en direct (aperçu), couleur/épaisseur/opacité de la barre flottante. Un glissement = une annotation `Ink` (un seul trait, un seul niveau d'annulation). |
| Texte libre | Clic (boîte 180×48 pt) ou glissement (boîte choisie) → boîte de dialogue (texte multiligne, taille, couleur, fond, bordure) ; le texte est renvoyé à la ligne à la largeur de la boîte, la boîte grandit vers le bas si le texte est plus haut. Double-clic en mode sélection = modifier. |
| Rectangle / Ellipse / Ligne / Flèche | Glisser ; trait, remplissage, épaisseur, opacité ; case « Flèche » pour l'outil Ligne. |
| Sélection d'une annotation existante | Clic (toute annotation non cachée, hors champs de formulaire et fenêtres contextuelles) : cadre + poignées ; glisser = déplacer ; poignées = redimensionner (annotations créées par SuniPDF) ; flèches = décaler (Maj ×10) ; Suppr = supprimer ; barre flottante = recolorer ; `Échap` = désélectionner. Surlignages/soulignements/barrés : sélection, recoloration, suppression, commentaire (pas de déplacement : ancrés au texte). |
| Liens cliquables | Outil par défaut (Sélection de texte) : le pointeur devient une main au-dessus d'un lien ; clic sur un lien interne = aller à la page ; clic sur un lien externe = **boîte de confirmation** montrant l'adresse complète (Ouvrir dans le navigateur / Copier / Annuler). Rien n'est ouvert sans clic explicite. Outil **Lien** : tracer une zone puis choisir « page N » ou « adresse web ». |
| Panneau Commentaires (F7) | Cartes groupées en fils : annotation, auteur, date, texte, **statut** (Aucun/Accepté/Refusé/Annulé/Terminé), réponses indentées, boutons Répondre / Supprimer le fil ; champ « Auteur » (mémorisé). Clic sur une carte = sélection sur la page. |
| Annuler / Rétablir, Enregistrer | Toutes les opérations sont des `EditOp` annulables (instantanés), enregistrées dans le PDF, relues par PDFium. |

Hors périmètre explicite (voir §2) : texte riche, gomme, polygones/nuages, tampons, formulaires, signatures.

---

## 2. Faisabilité

**Verdict : partiel** (cœur réalisable hors ligne avec les dépendances actuelles ; limites ci-dessous assumées).

| Fonction | Verdict | Remarque |
|---|---|---|
| Ink libre, Rectangle, Ellipse, Ligne/Flèche | oui | `/AP` explicite écrit par SuniPDF ; rendu confirmé par PDFium **[F6][F7]**. |
| Texte libre | oui, avec limites | Helvetica (police standard non incorporée, WinAnsi : accents latins, ponctuation courante, € etc. ; autres caractères remplacés par `?` dans l'apparence, le texte exact reste dans `/Contents`). Pas de gras/italique, pas de texte riche (`/RC`), pas de CJK/RTL. |
| Sélection/déplacement/redimensionnement/recoloration/suppression | oui, avec règle | Redimensionnement et changement de style de **forme/encre/ligne/texte libre** réservés aux annotations créées par SuniPDF (`/NM` préfixé `SuniPDF-`) : on ne régénère jamais l'apparence d'une annotation d'un autre producteur (pas de perte de fidélité). Déplacement : toutes sauf surlignages & co. Recoloration des surlignages/soulignements/barrés/ondulés/notes : oui (suppression de `/AP`, PDFium régénère **[F10]**). |
| Liens : lecture, clic, saut interne, URL externe confirmée | oui | Actions `Launch`, `GoToR`, JavaScript : affichées comme non prises en charge, jamais exécutées. Destination `/XYZ` : on va à la page seulement (la vue est page par page). Destinations nommées (chaînes) : non résolues. |
| Liens : création | oui | `/Dest [page /Fit]` ou `/A << /S /URI >>`. Pas de modification de la cible d'un lien existant (supprimer puis recréer). |
| Réponses + statuts | partiel | Écrits selon ISO 32000-1 (`/IRT`, `/RT /R`, `/StateModel /Review`, `/State`). Compatibilité Acrobat/Foxit **non testable ici**. Pas de modèle « Marked ». |
| Documents chiffrés | non (refus typé) | `lopdf` réécrit le fichier en clair ; ré-chiffrement hors tranche. Les surlignages/notes PDFium existants continuent de fonctionner sur ces documents. |

Laissé de côté volontairement : gomme d'encre, regroupement automatique de plusieurs traits, annotations Polygone/Polyligne/Nuage/Callout,
bordures pointillées, tampons (`/Stamp`), pièces jointes, rédaction (relève de `security-signatures`), champs de formulaire
(`/Widget`), mise à jour du lien par glisser, vue continue (plan `docs/plans/continuous-view.md`), correction de la rotation/CropBox pour
les outils **existants** (à ouvrir comme tâche séparée, cf. §10).

---

## 3. API moteur (`crates/sunipdf-core`)

Aucun nouveau crate (le cahier d'agent évoque `sunipdf-editor`, mais le dépôt héberge l'édition dans `sunipdf-core`, cf. `edit.rs`, D019).
Nouveaux modules : `geometry.rs`, `appearance.rs`, `annot_write.rs`, `links.rs`. Modules modifiés : `annot.rs`, `lib.rs`, `security.rs`
(visibilité de `lopdf_err`).

### 3.0 Conventions communes

* **Coordonnées** : `PointRect` (déjà dans `text.rs`) en *points de page affichée* (§3.1). Les fonctions de ce plan convertissent en
  espace utilisateur PDF juste avant d'écrire, et inversement à la lecture.
* **Écriture lopdf** : toujours via `Document::edit_pdf` (ci-dessous). Même schéma que `edit.rs:402` (`merge_content_streams`) :
  `snapshot()` → `lopdf` → `restore()`.
* **Aucune régénération de contenu de page PDFium** n'est nécessaire (on ne touche pas aux objets de page ; `regenerate_content`
  n'est donc pas appelé). `restore()` (`pages.rs:266`) recharge tout le document et rafraîchit les tailles.
* **Instantané/annulation** : l'appelant est `State::edit` (`worker.rs:372`) qui prend un instantané *avant* l'opération et le restaure
  en cas d'erreur. Les fonctions moteur échouent donc *avant* d'altérer `self.inner` : `restore()` ne remplace `self.inner` qu'après un
  chargement réussi (`pages.rs:266-274`).
* **Erreurs typées** (aucune chaîne UI) : `PageOutOfRange`, `SecurityRestricted` (document chiffré ou annotation verrouillée),
  `Other("annotation {i} not found")`, `Other("invalid rectangle")`, `Other("unsafe uri")`, `Other("empty text")`, etc. (même style que
  `edit.rs`). Pas de nouveau variant → `error_key` (`app.rs:1924`) et les codes CLI restent inchangés.
* **Date PDF** : fonction `pdf_date_now()` écrite à la main (algorithme `civil_from_days` de H. Hinnant, 12 lignes) → `D:YYYYMMDDHHmmSSZ`.
  Pas de `chrono` (qui n'est pas une dépendance directe de `sunipdf-core`).
* **Nom unique `/NM`** : `format!("SuniPDF-{:016x}", u64)` avec `getrandom::fill` (déjà utilisé à `security.rs:133`).
* **Texte PDF** : `lopdf::text_string(&str)` (`lopdf-0.45.0/src/common_data_structures/mod.rs:9`, ré-exporté `lib.rs:43` ; ASCII littéral, sinon
  UTF-16BE avec BOM via `encodings/mod.rs:191`) à l'écriture ; `lopdf::decode_text_string(&Object)` (`mod.rs:22`) à la lecture.
* **Objets lopdf** : `Dictionary::{new,get,get_mut,set,remove}` (`object.rs:344,352,364,370,386`), `Stream::new(dict, content)`
  (`object.rs:780`, fixe `/Length`), `Document::add_object` (`creator.rs:21`), `remove_object` (`creator.rs:36`),
  `get_pages` (`document.rs:533`), `get_dictionary(_mut)` (`document.rs:202,207`), `get_object(_mut)` (`document.rs:165,178`).
  Attention : `impl From<&str> for Object` produit un **Name** (`object.rs:106-110`) ; les chaînes PDF passent par `Object::string_literal`
  (`object.rs:137`) ou `text_string`.
* **Ne pas utiliser** `lopdf::Document::get_page_annotations` (`document.rs:747`) : il *saute* les entrées invalides, ce qui désaligne les
  indices avec ceux de PDFium. Indexer soi-même les entrées brutes de `/Annots`.

Garde de chiffrement et utilitaire d'édition (dans `annot_write.rs`) :

```rust
/// Refuse les documents chiffrés : lopdf::security::load retirerait /Encrypt (security.rs:78-95) et l'enregistrement
/// livrerait un fichier en clair. Le snapshot PDFium d'un document chiffré contient /Encrypt [F11].
fn load_for_edit(bytes: &[u8]) -> Result<lopdf::Document, EngineError> {
    let doc = lopdf::Document::load_mem(bytes).map_err(crate::security::lopdf_err)?;
    if doc.is_encrypted() {                      // lopdf document.rs:268
        return Err(EngineError::SecurityRestricted);
    }
    Ok(doc)
}

impl Document<'_> {
    /// Instantané → lopdf → rechargement PDFium. `f` reçoit le document lopdf, l'identifiant de l'objet page et la géométrie.
    fn edit_pdf<R>(
        &mut self,
        page_index: usize,
        f: impl FnOnce(&mut lopdf::Document, lopdf::ObjectId, &PageGeometry) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let geom = self.page_geometry(page_index)?;
        let bytes = self.snapshot()?;
        let mut pdf = load_for_edit(&bytes)?;
        let page_id = *pdf.get_pages().get(&(page_index as u32 + 1)).ok_or(EngineError::PageOutOfRange(page_index))?;
        let out = f(&mut pdf, page_id, &geom)?;
        self.restore(crate::security::save(pdf)?)?;
        Ok(out)
    }
}
```

Ajout d'une annotation et accès aux entrées brutes de `/Annots` (indices = ceux de PDFium, `FPDFPage_GetAnnot(i)` lit la i-ème entrée du
tableau, vérifié **[F12]**) :

```rust
fn annots_mut(pdf: &mut lopdf::Document, page: ObjectId) -> Result<&mut Vec<Object>, EngineError> {
    let indirect = pdf.get_dictionary(page).map_err(lopdf_err)?.get(b"Annots").ok().and_then(|o| o.as_reference().ok());
    match indirect {
        Some(id) => pdf.get_object_mut(id).and_then(Object::as_array_mut).map_err(lopdf_err),
        None => {
            let dict = pdf.get_dictionary_mut(page).map_err(lopdf_err)?;
            if !matches!(dict.get(b"Annots"), Ok(Object::Array(_))) { dict.set("Annots", Object::Array(vec![])); }
            dict.get_mut(b"Annots").and_then(Object::as_array_mut).map_err(lopdf_err)
        }
    }
}
fn push_annotation(pdf: &mut lopdf::Document, page: ObjectId, mut dict: Dictionary, ap: Option<Stream>) -> Result<ObjectId, EngineError> {
    if let Some(stream) = ap { let ap_id = pdf.add_object(stream); dict.set("AP", lopdf::dictionary! { "N" => ap_id }); }
    dict.set("P", page);                                   // From<ObjectId> for Object : Reference
    let id = pdf.add_object(dict);
    annots_mut(pdf, page)?.push(Object::Reference(id));
    Ok(id)
}
/// Base commune : Type, Subtype, Rect (espace utilisateur), F 4 (imprimable), NM, M, CreationDate, T (auteur facultatif).
fn base_annotation(subtype: &str, user_rect: PointRect, author: Option<&str>) -> Dictionary;
```

### 3.1 `geometry.rs` — géométrie de page (rotation, CropBox)

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry { pub rotation: Rotation, pub crop: PointRect /* zone visible, espace utilisateur */ }
impl PageGeometry {
    pub fn display_size(&self) -> (f32, f32);                     // (W,H) ou (H,W) selon la rotation
    pub fn display_to_user(&self, x: f32, y: f32) -> (f32, f32);
    pub fn user_to_display(&self, x: f32, y: f32) -> (f32, f32);
    pub fn rect_display_to_user(&self, r: PointRect) -> PointRect;   // deux coins opposés puis normalisation min/max
    pub fn rect_user_to_display(&self, r: PointRect) -> PointRect;
}
impl Document<'_> { pub fn page_geometry(&self, page_index: usize) -> Result<PageGeometry, EngineError>; }
```

Formules (W = crop.right − crop.left, H = crop.top − crop.bottom ; u = x − crop.left, v = y − crop.bottom ; Hd = hauteur affichée) —
**vérifiées sur PDFium 7881 pour les 8 combinaisons rotation × CropBox [F8]** :

| `/Rotate` | utilisateur → affichage (xd, yd) | affichage → utilisateur (u, v) | taille affichée |
|---|---|---|---|
| 0 | (u, v) | (xd, yd) | (W, H) |
| 90 | (v, W − u) | (W − yd, xd) | (H, W) |
| 180 | (W − u, H − v) | (W − xd, H − yd) | (W, H) |
| 270 | (H − v, u) | (yd, H − xd) | (H, W) |

Appels de bibliothèque : `Document::page_rotation` (`pages.rs:151` → `PdfPage::rotation`, `pdfium-render/src/pdf/document/page.rs:292`) ;
CropBox : `page.boundaries().crop()` (`boundaries.rs:220`, renvoie une erreur s'il n'y a pas de `/CropBox`) avec repli
`boundaries().media()` (`boundaries.rs:113`) ; `PdfPageBoundaryBox.bounds: PdfRect` (`boundaries.rs:27-30`) ; coordonnées `left()/bottom()/right()/top()`
(`rect.rs:119…`) puis `.value`. La zone visible est l'intersection CropBox ∩ MediaBox (c'est ce que PDFium utilise pour la taille de page ;
vérifié : tailles rendues 792×612 / 672×512 identiques à `display_size`). Repli final : `(0, 0, w, h)` déduit de `page_size`
(permuter si 90/270).

### 3.2 `appearance.rs` — fonctions pures (aucun appel PDFium, testables sans moteur)

```rust
pub(crate) fn fmt_num(v: f32) -> String;                         // 3 décimales max, sans zéros finaux ni exposant
pub(crate) fn color_ops(rgb: (u8, u8, u8), stroke: bool) -> String;   // "r g b RG" / "r g b rg" (composantes /255)
pub fn helvetica_width(c: char) -> u16;                          // table ci-dessous, 556 par défaut
pub fn text_width(s: &str, size_pt: f32) -> f32;
pub fn wrap_text(text: &str, max_width_pt: f32, size_pt: f32) -> Vec<String>;   // sauts de ligne conservés, mots trop longs coupés
pub(crate) fn winansi_literal(line: &str) -> Vec<u8>;            // contenu entre parenthèses : \ ( ) échappés, octets >0x7E en \ddd, inconnu → '?'
pub fn simplify_polyline(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)>;   // Ramer-Douglas-Peucker itératif (pile explicite)
pub(crate) struct Ap { pub w: f32, pub h: f32, pub content: Vec<u8>, pub opacity: f32, pub uses_font: bool }
pub(crate) fn square_ap(w: f32, h: f32, s: &AnnotStyle) -> Ap;
pub(crate) fn circle_ap(w: f32, h: f32, s: &AnnotStyle) -> Ap;
pub(crate) fn ink_ap(w: f32, h: f32, strokes_local: &[Vec<(f32, f32)>], s: &AnnotStyle) -> Ap;
pub(crate) fn line_ap(w: f32, h: f32, p1: (f32, f32), p2: (f32, f32), arrow: bool, s: &AnnotStyle) -> Ap;
pub(crate) fn free_text_ap(w: f32, h: f32, lines: &[String], s: &FreeTextStyle) -> Ap;
```

**Table des largeurs Helvetica** (1/1000 em, ASCII 0x20…0x7E, 95 valeurs, ordre ASCII) — validée contre PDFium (somme des 95 glyphes
600,7 pt à 12 pt contre 600,19 pt mesurés par `FPDFPageObj_GetBounds` ; l'écart est celui des approches de glyphe) **[F15]** :

```
278 278 355 556 556 889 667 191 333 333 389 584 278 333 278 278   // espace ! " # $ % & ' ( ) * + , - . /
556 556 556 556 556 556 556 556 556 556                           // 0-9
278 278 584 584 584 556 1015                                      // : ; < = > ? @
667 667 722 722 667 611 778 722 278 500 667 556 833 722 778 667 778 722 667 611 722 667 944 667 667 611   // A-Z
278 278 278 469 556 333                                           // [ \ ] ^ _ `
556 556 500 556 556 278 556 556 222 222 500 222 833 556 556 556 556 333 500 278 556 500 722 500 500 500   // a-z
334 260 334 584                                                   // { | } ~
```

Caractères Latin-1/WinAnsi hors ASCII : largeur 556 (les lettres accentuées valent leur lettre de base à ±10 % ; marge de 2 % sur la largeur
disponible pour le renvoi à la ligne). Encodage WinAnsi : U+0020–U+007E et U+00A0–U+00FF directs ; U+20AC→0x80, U+201A→0x82, U+0192→0x83,
U+201E→0x84, U+2026→0x85, U+2020→0x86, U+2021→0x87, U+02C6→0x88, U+2030→0x89, U+0160→0x8A, U+2039→0x8B, U+0152→0x8C, U+017D→0x8E,
U+2018→0x91, U+2019→0x92, U+201C→0x93, U+201D→0x94, U+2022→0x95, U+2013→0x96, U+2014→0x97, U+02DC→0x98, U+2122→0x99, U+0161→0x9A, U+203A→0x9B,
U+0153→0x9C, U+017E→0x9E, U+0178→0x9F ; le reste → `?`.

Flux d'apparence (tous : `BBox [0 0 w h]`, origine locale ; l'annotation est positionnée par `/Rect` — c'est la règle 8.1 de la spec, confirmée
par PDFium **[F7]** ; ainsi un déplacement ne modifie que `/Rect` et les tableaux de géométrie). Dictionnaire du flux : `/Type /XObject /Subtype /Form
/BBox /Resources` avec `/ExtGState << /GS << /Type /ExtGState /CA a /ca a >> >>` (a = opacité) et, si `uses_font`,
`/Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >> >>`. Contenus :

* **Square** : `q /GS gs [fill rg] stroke RG lw w  lw/2 lw/2 (w-lw) (h-lw) re  B|S|f Q` (`B` si fond et trait, `S` trait seul, `f` si `lw = 0`).
* **Circle** : même en-tête ; ellipse inscrite à `lw/2` du bord en 4 courbes de Bézier (κ = 0,5522847498) :
  `cx+rx cy m`, puis `c` vers (cx, cy+ry), (cx−rx, cy), (cx, cy−ry), (cx+rx, cy), `h`, `B|S|f`.
* **Ink** : `q /GS gs stroke RG lw w 1 J 1 j` puis pour chaque trait `x y m x y l … S` (un trait d'un seul point donne un point rond grâce
  à `1 J`) `Q`. Points en coordonnées locales (− `rect.left`, − `rect.bottom`). Lissage optionnel (courbes quadratiques passant par les milieux
  de segments) — non requis au premier jet.
* **Line** : P1→P2 ; flèche ouverte en P2 de longueur `hl = max(8, 4·lw)`, demi-angle 25° : `P1 m P2 l S` puis `A m P2 l B l S` où A,B = P2 − d·hl ± n·hl·0,466
  (d vecteur unitaire, n sa normale). Longueur < 1 pt → erreur `Other("line too short")`.
* **FreeText** : `q /GS gs` ; fond `[fill rg 0 0 w h re f]` ; bordure `[textcolor RG bw w bw/2 bw/2 (w-bw) (h-bw) re S]` ; puis
  `q 0 0 w h re W n BT /Helv size Tf textcolor rg pad (h - pad - 0.8·size) Td (ligne1) Tj 0 -1.2·size Td (ligne2) Tj … ET Q Q` avec `pad = 2 + bw`.
  (Le rendu PDFium d'une telle apparence a été vérifié, texte bleu sur deux lignes **[F7]**.)

### 3.3 `annot_write.rs` — types publics (ré-exportés dans `lib.rs`)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotStyle { pub rgb: (u8, u8, u8), pub fill_rgb: Option<(u8, u8, u8)>, pub width_pt: f32, pub opacity: f32 }   // défaut : rouge DC2626, 2 pt, opacité 1
#[derive(Debug, Clone, PartialEq)]
pub struct FreeTextStyle { pub size_pt: f32, pub rgb: (u8, u8, u8), pub fill_rgb: Option<(u8, u8, u8)>, pub border_pt: f32 }  // défaut : 12 pt, noir, sans fond, 0
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotShape { Square, Circle, Line, Arrow }
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget { Page(usize), Uri(String), Unsupported }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewStatus { #[default] None, Accepted, Rejected, Cancelled, Completed }   // + id()/from_id() ("none","accepted",…) et pdf_name()
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotDetail {
    pub index: usize,                 // position dans /Annots == index PDFium
    pub subtype: String,              // "Square", "Ink", "FreeText", "Link", "Text", "Highlight", "Popup"…
    pub rect: PointRect,              // espace page affichée
    pub contents: Option<String>, pub author: Option<String>, pub modified: Option<String>, pub name: Option<String>,
    pub rgb: Option<(u8, u8, u8)>, pub fill_rgb: Option<(u8, u8, u8)>, pub width_pt: Option<f32>, pub opacity: f32,
    pub parent: Option<usize>,        // /IRT résolu en index
    pub status: Option<ReviewStatus>, // seulement pour les annotations d'état (/StateModel /Review)
    pub link: Option<LinkTarget>,
    pub hidden: bool, pub locked: bool, pub ours: bool,           // ours = /NM commence par « SuniPDF- »
    pub can_move: bool, pub can_resize: bool, pub can_restyle: bool, pub can_edit_text: bool,
    pub replies: usize,               // nombre de descendants /IRT (pour « Supprimer le fil »)
}
```

Fonctions (`impl Document<'_>`). Toutes prennent des coordonnées de **page affichée**, passent par `edit_pdf` et renvoient `Result<(), EngineError>`
sauf indication ; `author: Option<&str>` écrit `/T` quand il est non vide.

| Signature | Écriture lopdf (clés) | Notes |
|---|---|---|
| `add_ink_annotation(&mut self, page, strokes: &[Vec<(f32,f32)>], style: &AnnotStyle, author) ` | `Subtype Ink`, `InkList [[x y …] …]`, `Rect` = boîte des points ± (lw/2+0,5), `C`, `BS << /W lw >>`, `CA`, `AP` = `ink_ap` | Chaque trait : `simplify_polyline(ε = 0,4 pt)` **avant** conversion ; plafonds 4 000 points/trait et 64 traits ; moins de 2 points au total → `Other("empty stroke")`. |
| `add_shape_annotation(&mut self, page, shape: AnnotShape, rect: PointRect, style: &AnnotStyle, author)` | `Square`/`Circle`: `Rect`, `C`, `IC` (si fond), `BS`, `CA`, `AP`. `Line`/`Arrow`: `L [x1 y1 x2 y2]`, `LE [/None /None]` ou `[/None /OpenArrow]`, `Rect` englobant + marge, `C`, `BS`, `CA`, `AP` | Pour `Line`/`Arrow`, `rect` donne les extrémités (comme `edit::add_shape`) : (left,bottom) → (right,top). Taille minimale 2×2 pt. |
| `add_free_text_annotation(&mut self, page, rect: PointRect, text: &str, style: &FreeTextStyle, author)` | `Subtype FreeText`, `Rect`, `Contents` (`text_string`), `DA "/Helv {s} Tf {r g b} rg"`, `Q 0`, `C` (fond), `BS`, `AP` = `free_text_ap` | Texte vide → `Other("empty text")`. Hauteur étendue vers le bas si nécessaire (`needed = lignes·1,2·s + 2·pad`). Largeur < 2·taille → erreur. Le dictionnaire reste exploitable par Acrobat (DA + AP) **[F5]** (l'apparence générée par PDFium n'est pas utilisable). |
| `add_link_annotation(&mut self, page, rect: PointRect, target: &LinkTarget)` | `Subtype Link`, `Rect`, `Border [0 0 0]`, `Dest [pageRef /Fit]` ou `A << /S /URI /URI (…) >>` | `Page(n)` hors plage → `PageOutOfRange(n)` ; `Uri` refusée si `!is_safe_external_uri` (§3.6) ; `Unsupported` → erreur. Pas d'`AP`. |
| `annotation_details(&self, page) -> Result<Vec<AnnotDetail>, EngineError>` | lecture seule (snapshot + lopdf, **pas** de `restore`) | Voir §3.4. Coûteux (document entier) : à appeler à la demande. |
| `move_annotation(&mut self, page, index, dx, dy)` | `Rect`, `QuadPoints`, `InkList` (imbriqué), `L`, `CL`, `Vertices` translatés du vecteur utilisateur ; `M` mis à jour ; `AP` inchangé | Delta d'affichage converti : `display_to_user(dx,dy) − display_to_user(0,0)`. Refus si `can_move == false` (`Other("annotation cannot be moved")`, `Locked` → `SecurityRestricted`). |
| `resize_annotation(&mut self, page, index, rect: PointRect)` | `Rect` + changement d'échelle affine ancien→nouveau rect de `InkList`/`L`, puis **régénération de l'AP** | Réservé à `ours` & `can_resize` ; taille minimale 4 pt ; texte libre : nouveau renvoi à la ligne, hauteur jamais inférieure au texte. |
| `set_annotation_style(&mut self, page, index, style: &AnnotStyle)` | `ours` Square/Circle/Line/Ink/FreeText : `C`, `IC`, `BS`, `CA` + régénération de l'AP (FreeText : `rgb` = texte, `fill_rgb` = fond, `width_pt` = bordure). Surlignage/Soulignement/Barré/Ondulé/Note : `C` et `CA`, **suppression de `AP` et de `PDFIUM_HasGeneratedAP`** puis `ensure_appearances` | Recoloration validée **[F10]**. |
| `set_annotation_contents(&mut self, page, index, text: &str)` | `Contents` (+`M`) ; texte libre `ours` : régénération de l'AP | `can_edit_text`. |
| `delete_annotation_thread(&mut self, page, index)` | retrait de l'entrée de `/Annots` (indices décroissants), de ses descendants `/IRT` (transitifs) et des `Popup` dont `/Parent` est supprimé ; `remove_object` sur les dictionnaires retirés | Le `delete_annotation` PDFium existant (`annot.rs:198`) reste pour les annotations sans réponses (et les documents chiffrés). |
| `add_reply(&mut self, page, parent: usize, text: &str, author) -> Result<(), _>` | `Subtype Text`, `Rect` 20×20 pt au coin haut-gauche du parent, `Contents`, `T`, `IRT` (référence du parent), `RT /R`, `Name /Comment`, `F 4`, `AP` = flux vide (`BBox [0 0 1 1]`, contenu vide) | L'AP vide empêche PDFium de dessiner une icône de note par réponse. Le parent doit être un objet indirect. |
| `set_review_status(&mut self, page, parent: usize, status: ReviewStatus, author)` | annotation d'état : `Subtype Text`, `IRT`, `RT /R`, `StateModel /Review`, `State /Accepted…`, `T`, `AP` vide | Le statut courant d'un parent = `State` de la **dernière** annotation d'état de ses descendants (ordre du tableau). |
| `ensure_appearances(&self, page)` (public) | aucune | Rend la page en ~8 px de large (`render_page(page, 8.0 / largeur)`) : PDFium génère et mémorise les `/AP` manquants **[F4]**. Indispensable avant un `snapshot()` quand on vient de supprimer un `/AP`. |

Tous les chemins qui écrivent des annotations sur un document chiffré renvoient `SecurityRestricted` **avant** toute modification.

### 3.4 Lecture : `annotation_details`

1. `snapshot()` → `load_for_edit`-like (mais en lecture, document chiffré refusé aussi).
2. Entrées brutes de `/Annots` : pour chaque position `i`, résoudre l'entrée (référence ou dictionnaire direct) ; entrée invalide → ligne
   `subtype = "Unknown"`, tous les `can_* = false` (l'indice est conservé).
3. `id_to_index: HashMap<ObjectId, usize>` ; `/IRT` → `parent`. Page cible d'un `Dest` : inversion de `get_pages()` (id → numéro−1).
4. Couleurs `/C` et `/IC` (1 = gris, 3 = RVB, 4 = CMJN approximé) ; largeur : `/BS << /W >>` sinon `/Border[2]` ; `CA` défaut 1.
5. `link` : `/Dest [ref …]` ou `/A << /S /GoTo /D [ref …] >>` → `Page` ; `/A << /S /URI /URI (…) >>` → `Uri` (octets en UTF-8 lossy) ; autre → `Unsupported`.
6. Droits : `ours` = `/NM` préfixé ; `locked` = bit 128 de `/F` ; `hidden` = bit 2.
   `can_move` = !locked ∧ sous-type ∈ {Square, Circle, Line, Ink, FreeText, Text, Link, Stamp} ; `can_resize` = ours ∧ sous-type ∈ {Square, Circle, Line, Ink, FreeText, Link} ;
   `can_restyle` = ¬locked ∧ ((ours ∧ sous-type ∈ {Square, Circle, Line, Ink, FreeText}) ∨ sous-type ∈ {Highlight, Underline, StrikeOut, Squiggly, Text}) ;
   `can_edit_text` = ¬locked ∧ sous-type ∉ {Popup, Widget, Link} ∧ (¬FreeText ∨ ours).
7. `rect` converti en page affichée par `rect_user_to_display`.

### 3.5 `annot.rs` (modifié)

* `AnnotationInfo` reçoit `author: Option<String>` (`creator()`, `annotation.rs:910/1159`) et `modified: Option<String>`
  (`modification_date()`, `:922/1179`) ; `rect` passe en **page affichée** (changement de sémantique documenté : l'UI n'utilisait que `kind`/`contents`/`index`).
* `annotations()` itère `0..annotations.len()` avec `get(i)` (`annotations.rs:92`) au lieu de `iter()` (`annotations.rs:619-628` s'arrête à la première entrée invalide) ; une entrée
  illisible produit une ligne `kind = "Unknown"` pour garder l'alignement des indices.
* `annotation_at(&self, page, x, y) -> Result<Option<AnnotationInfo>, _>` (tolérance 2 pt, dernière dans l'ordre du tableau = dessus, ignore `Popup`, `Widget`,
  annotations cachées `is_hidden()` `annotation.rs:954/1230`).
* **Ne jamais appeler** `stroke_color()/fill_color()/set_stroke_color()/set_fill_color()` de `pdfium-render` sur une annotation qui a un `/AP` : en cas d'échec de
  `FPDFAnnot_GetColor/SetColor` ils retombent sur `FPDFPageObj_*Color(self.handle() as FPDF_PAGEOBJECT, …)` (`private.rs:404-416, 442-460, 488-500, 526-540`), un
  transtypage d'un handle d'annotation en objet de page (comportement indéfini). L'usage actuel d'`add_markup` (couleur posée juste après création, avant tout rendu) reste correct.

### 3.6 `links.rs` — lecture des liens et sécurité des URL

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LinkInfo { pub index: usize /* index d'annotation */, pub rect: PointRect /* page affichée */, pub target: LinkTarget }
impl Document<'_> { pub fn page_links(&self, page_index: usize) -> Result<Vec<LinkInfo>, EngineError>; }
pub fn is_safe_external_uri(uri: &str) -> bool;
```

`page_links` : `page.annotations()` (`page.rs:390`) → pour chaque `get(i)` (`annotations.rs:92`), `annotation.as_link_annotation()` (`annotation.rs:588`) → `.link()`
(`link.rs:48`, `FPDFAnnot_GetLink`) → cible : `link.destination()` (`link.rs:69`) puis `.page_index()` (`destination.rs:276`, `c_int` ≥ 0) = `Page`, sinon
`link.action()` (`link.rs:51`) → `.as_uri_action()` (`action.rs:241`) → `.uri()` (`action/uri.rs:30`) = `Uri`, sinon `Unsupported`. `/Dest` et `/A /GoTo` donnent la même page **[F9]**.
**Ne pas** utiliser `page.links().iter()/get()` (`links.rs:115-129,193`) : `FPDFLink_Enumerate` reçoit un *indice d'annotation*, d'où des doublons (expérience : 4 résultats pour 2 liens) et un `len()` faux.

`is_safe_external_uri` : après `trim`, 1 ≤ longueur ≤ 2048, tous les caractères ASCII imprimables sans espace (0x21–0x7E), schéma (insensible à la casse) ∈ {`http`, `https`, `mailto`} ;
pour http(s) : `://` suivi d'un hôte non vide ne commençant ni par `/` ni par `\`. Refuse `file:`, `javascript:`, `ms-*:`, chemins `C:\…`, `\\serveur\…`, `data:`, chaînes vides ou à caractères de contrôle.
Utilisée à la création (moteur), au clic (UI) et par la CLI. L'ouverture elle-même est faite par l'UI : `ctx.open_url(egui::OpenUrl::new_tab(url))` (`egui-0.36.2/src/context.rs:1677`,
`data/output.rs:281`) → `egui-winit-0.36.2/src/lib.rs:1130,1394-1400` → `webbrowser::open` (feature `links` d'`eframe`, déjà activée au `Cargo.toml` racine) qui passe l'adresse au navigateur
par défaut (`webbrowser-1.2.4/src/windows.rs:28-92`, **sans** filtrage de schéma sauf feature `hardened`, `lib.rs:318-325`, non activée) : la liste d'autorisation est donc obligatoire **[F14]**.

### 3.7 `lib.rs`

Déclarer `pub mod geometry; mod appearance; pub mod annot_write; pub mod links;` et ré-exporter :
`AnnotStyle, FreeTextStyle, AnnotShape, AnnotDetail, LinkTarget, LinkInfo, ReviewStatus, PageGeometry, is_safe_external_uri`.

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

**Nouveaux `EditOp`** (enum à `worker.rs:76`, tous annulables via `State::edit`, `worker.rs:372`) :

```rust
AddInk { page: usize, strokes: Vec<Vec<(f32, f32)>>, style: AnnotStyle, author: Option<String> },
AddAnnotShape { page: usize, shape: AnnotShape, rect: PointRect, style: AnnotStyle, author: Option<String> },
AddFreeText { page: usize, rect: PointRect, text: String, style: FreeTextStyle, author: Option<String> },
AddLink { page: usize, rect: PointRect, target: LinkTarget },
MoveAnnotation { page: usize, index: usize, dx: f32, dy: f32 },
ResizeAnnotation { page: usize, index: usize, rect: PointRect },
StyleAnnotation { page: usize, index: usize, style: AnnotStyle },
SetAnnotationText { page: usize, index: usize, text: String },
DeleteAnnotationThread { page: usize, index: usize },
Reply { page: usize, parent: usize, text: String, author: Option<String> },
SetReviewStatus { page: usize, parent: usize, status: ReviewStatus, author: Option<String> },
```

`apply` (`worker.rs:415`) : un bras par variante, par exemple `EditOp::AddInk { page, strokes, style, author } => doc.add_ink_annotation(page, &strokes, &style, author.as_deref())`.
`EditOp::DeleteAnnotation` (déjà là, `worker.rs:457`) est conservé pour les annotations sans réponse.

**Nouvelles `Request`** (`worker.rs:178`) :
* `Annotations { page, detailed: bool }` (remplace `Annotations { page }`, gérée à `worker.rs:795`) : liste PDFium rapide (`doc.annotations`) ; si `detailed` **et** si la page contient au moins une annotation autre que `Link`/`Popup`/`Widget`,
  `doc.annotation_details(page).ok()` dans le même tour de boucle (évite de bloquer le thread moteur sur les pages sans annotation).
* `Links { page }` → `doc.page_links(page)`.

**Nouvelles `Response`** (`worker.rs:238`) : `Annotations { page, list: Vec<AnnotationInfo>, details: Option<Vec<AnnotDetail>> }` (remplace la variante actuelle ; consommateur `app.rs:473`) et `Links { page, list: Vec<LinkInfo> }`.

**`BatchOp`** : aucune variante (aucun fichier produit ; ces opérations modifient le document ouvert et sont annulables). **`DocState`** inchangé.

**Cohérence onglets** : les instantanés d'annulation par onglet existent déjà (`SuspendedDoc`, `worker.rs:21`) ; aucune adaptation.

---

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Fichiers à toucher

| Fichier | Modification |
|---|---|
| `src/annot_adv.rs` (**nouveau**, déclaré dans `main.rs:3-15`) | État `AnnotAdvState`, outils de création (encre, texte libre, formes, lien), sélection d'annotation, clic sur lien, dialogues (texte libre, lien, réponse, confirmation d'URL), aperçu d'encre. Regrouper ici limite les conflits avec `docs/plans/continuous-view.md` qui réécrira `document_area`. |
| `annotate.rs` | `Tool` (+7 variantes, `ALL: [Tool; 11]` → `[Tool; 18]` à `annotate.rs:28`, `command()` `:42`), `AnnotateState` (+`adv: AnnotAdvState`, `:88`), `finish_drag` (`:190`), palette défilante (`:108`), panneau Commentaires en fils (`:303`), `annotation_label` (`:390`). |
| `editor.rs` | `Tool::is_edit()` (`:154`) étendu aux nouveaux outils ; `corner_at` (`:175`), `preview_bounds` (`:232`), `paint_object_selection` (`:532`) généralisés par `fn selection_bounds(&self, page) -> Option<(PointRect, bool /*proportions verrouillées*/)>` (objet **ou** annotation) ; `page_cursor` (`:186`) ; `object_bar` (`:609`) : opacité, case Flèche, branche « annotation sélectionnée » ; `delete_selected_object` (`:522`) et `nudge_selected` (`:483`) : traitent l'annotation sélectionnée d'abord. |
| `app.rs` | `handle_responses` (`:327`) : `Response::Annotations`/`Links` (`:473`), `DocumentChanged` (`:422`) invalide `details_page`, `links_page` et redemande la sélection ; événements de glissement (`:1187-1233`) : points d'encre, `Click` sur lien/annotation ; `execute` (`:793`, bras des outils `:923`) ; `handle_input` (`:576`) : raccourcis `A`, `D`, `F` (ligne 739-749), et prompts (`:599`) ; appel des dialogues après `:1881` ; `refresh_annotations_if_needed` appelée aussi pour l'outil de sélection. |
| `shell.rs` | `ToolTab::Annotate` (`:88`), `ToolTab::All` (`:48`), `command_color` (`:150`), `command_icon` (`:246`, `match` **exhaustif**). |
| `settings.rs` | `author: String` (`#[serde(default)]`, vide par défaut). |
| `worker.rs`, `tabs.rs` | cf. §4 ; `tabs.rs` : rien (l'état vit dans `AnnotateState`, déjà échangé par `take_tab`/`put_tab`, `tabs.rs:52,82`). |
| `crates/sunipdf-core/src/command.rs` | 7 variantes `Command` (enum `:6`) + 7 entrées `REGISTRY` (après `ToolNote`, `:391`). |
| `assets/i18n/*.json` | §7. |

### 5.2 Commandes à ajouter (registre `command.rs`)

| `Command` | id | clé libellé | raccourci (clé) | menu | `ToolTab` | icône Fluent (repli) |
|---|---|---|---|---|---|---|
| `ToolSelectAnnot` | `tool.select_annot` | `menu.edit.tool_select_annot` | `A` (`shortcut.tool_select_annot`) | Edit | Annotate | `\u{E7C9}` (`↖`) |
| `ToolFreeText` | `tool.free_text` | `menu.edit.tool_free_text` | `F` | Edit | Annotate | `\u{E8BD}` (`T`) |
| `ToolInk` | `tool.ink` | `menu.edit.tool_ink` | `D` | Edit | Annotate + Tous | `\u{E70F}` (`✎`) |
| `ToolAnnotRect` | `tool.annot_rect` | `menu.edit.tool_annot_rect` | aucun | Edit | Annotate | `glyph::RECT` |
| `ToolAnnotEllipse` | `tool.annot_ellipse` | `menu.edit.tool_annot_ellipse` | aucun | Edit | Annotate | `glyph::ELLIPSE` |
| `ToolAnnotLine` | `tool.annot_line` | `menu.edit.tool_annot_line` | aucun | Edit | Annotate | `glyph::LINE` |
| `ToolLink` | `tool.link` | `menu.edit.tool_link` | aucun | Edit | Annotate + Tous | `\u{E71B}` (`🔗`) |

Les points de code E70F, E71B, E7C9, E8BD existent dans `C:\Windows\Fonts\SegoeIcons.ttf` de ce poste (vérifié dans la table `cmap`) ; leur signification visuelle est à valider par capture
(étape E3). Ajouter les constantes dans `icons.rs::glyph` (`INK`, `FREE_TEXT`, `LINK`, `SELECT_ANNOT`). `needs_document = true` pour toutes (comme `tool.note`). Couleur : `theme::CAT_ANNOTATE`
(arm de `command_color`, `shell.rs:152-159`). Aucun raccourci ne collisionne : `A`, `D`, `F` sans modificateur sont libres (`Ctrl+D` Propriétés, `Ctrl+F` Rechercher sont consommés plus haut dans la
même fermeture `app.rs:699-774`, placer les nouvelles lignes `key(Modifiers::NONE, …)` **après** elles, comme `R`/`Ctrl+R`).

Pas de commande `delete_annotation` : la commande `DeleteObject` (touche Suppr, `app.rs:750`) supprime l'annotation sélectionnée lorsque l'outil est « Sélectionner une annotation ».

### 5.3 Pièges de compilation (match exhaustifs à compléter ensemble)

`Command` (enum + `REGISTRY` — sinon panic `expect("toute commande est enregistrée")`, `command.rs:614`), `command_icon` (`shell.rs:246`), `Tool::ALL` (taille du tableau) et `Tool::command`,
`App::execute` (liste des outils `app.rs:923-941`), `finish_drag` (`annotate.rs:190`), `page_cursor` (`editor.rs:186`), `annotation_label` (valeur par défaut déjà prévue). Test existant `registry_keys_exist_in_both_catalogs` (`app.rs:1984`)
vérifie les clés des commandes.

### 5.4 Conception des interactions

État (dans `annot_adv.rs`, champ `adv` de `AnnotateState` donc échangé par onglet) :

```rust
#[derive(Default)]
pub struct AnnotAdvState {
    pub ink: Vec<(f32, f32)>,                          // trait en cours (points de page affichée)
    pub free_text: Option<FreeTextPrompt>,             // { page, rect, index: Option<usize>, text, size, rgb:[u8;3], fill:bool, fill_rgb, border, focus }
    pub link: Option<LinkPrompt>,                      // { page, rect, to_page: bool, page_number: usize, url: String, error: Option<&'static str> }
    pub reply: Option<ReplyPrompt>,                    // { page, parent, text, focus }
    pub url_confirm: Option<String>,                   // adresse en attente de confirmation
    pub details: Vec<AnnotDetail>, pub details_page: Option<usize>,
    pub selected: Option<(usize, AnnotDetail)>,        // (page, annotation)
    pub reselect: Option<(usize, usize)>,              // resélection après modification (indice stable)
    pub links: Vec<LinkInfo>, pub links_page: Option<usize>,
}
```

Préférences globales ajoutées à `EditState` (`editor.rs:~75`, non échangées entre onglets, comme les couleurs de forme) : `annot_opacity: f32` (défaut 1,0), `annot_arrow: bool`, `free_text_size: f32` (12).

* **Dessin (Ink)** : `DragEvent::Start` (`app.rs:1188`) → `adv.ink.clear()` + point de départ ; `Move` (`:1197`) → push du point si distance > 0,5 pt ; `Stop` → `finish_drag` →
  si ≥ 2 points : `Request::Edit(EditOp::AddInk { strokes: vec![points], style (couleur/épaisseur/opacité de la barre), author })`. L'aperçu (polyligne lissée, couleur et épaisseur réelles) est dessiné par
  `paint_annot_preview`, appelée à côté de `paint_object_selection` (appel à `app.rs:1144`).
* **Rectangle/Ellipse/Ligne (annotations)** : copie du comportement de `Tool::Rectangle/Ellipse/Line` (`editor.rs:412-440`) avec `EditOp::AddAnnotShape` ; l'aperçu réutilise `paint_object_selection` (`editor.rs:532`) en étendant le `match self.annotate.tool` (`Rectangle | AnnotRect`, …). Glissement < 2 pt : ignoré.
* **Texte libre** : `finish_drag` / `DragEvent::Click` (`app.rs:1207`) → `adv.free_text = Some(FreeTextPrompt …)` (clic : boîte 180×48 pt dont le coin haut-gauche est sous le curseur) ; dialogue `dialog::show` (même gabarit que `text_prompt_dialog`,
  `editor.rs:711`) : champ multiligne, taille (`DragValue` 6–96), couleur, « Fond » (case + couleur), « Bordure » (`DragValue` 0–10) ; validation `Ctrl+Entrée`, annulation `Échap`. Mode édition (`index: Some`) : ouvert par double-clic sur une annotation texte libre `ours` ou F2 →
  `EditOp::SetAnnotationText` (+ `StyleAnnotation` si changement de style).
* **Lien** : zone ≥ 6×6 pt → `LinkPrompt` ; dialogue `dialog::show` : deux boutons radio (Page / Adresse web), `DragValue` page 1…N ou champ URL ; validation `is_safe_external_uri` côté UI (message `dialog.link.invalid_url`) puis `EditOp::AddLink`. Pendant l'outil Lien et l'outil de sélection, les zones de lien du cache sont dessinées en pointillés bleus.
* **Clic sur lien** (outil `Tool::Select`, `DragEvent::Click`, `app.rs:1207`) : test local dans `adv.links` (cache rempli par `Request::Links` à chaque changement de page et après `DocumentChanged`) ; `Page(n)` → `self.go_to_page(n)` (`app.rs:268`) ; `Uri(u)` → `adv.url_confirm = Some(u)` si `is_safe_external_uri`, sinon message d'état « lien non pris en charge » ; `Unsupported` → message d'état. `page_cursor` renvoie `CursorIcon::PointingHand` quand le pointeur est sur un lien (outil Select uniquement).
* **Boîte de confirmation d'URL** (`dialog::show`, `Spec::warning()` — `dialog.rs:50`) : sous-titre « SuniPDF fonctionne hors ligne… », corps = adresse complète (sélectionnable, retours à la ligne), pied : **Ouvrir dans le navigateur** (`dialog::primary`) → `ctx.open_url(egui::OpenUrl::new_tab(url))`, **Copier l'adresse** (`dialog::secondary` → `ctx.copy_text`), **Annuler** (`Échap`). Ajouter la fenêtre aux gardes de `handle_input` (`app.rs:591-597`) pour couper les raccourcis tant qu'elle est ouverte.
* **Sélection d'annotation** (`Tool::SelectAnnot`) : `Click`/`Start` → `annotation_at` **local** sur `adv.details` (ou, à défaut, la liste rapide `annotate.annotations`) : dernière annotation contenant le point (tolérance 2 pt), hors `Popup`/`Widget`/cachées. Le cadre et les 4 poignées réutilisent `Corner`/`DragMode`/`HANDLE_PX` (`editor.rs:15-110`) via `selection_bounds` ; le glissement de déplacement envoie `EditOp::MoveAnnotation`, celui d'une poignée `EditOp::ResizeAnnotation` (désactivés si `!can_move`/`!can_resize`, curseur normal). `adv.reselect = Some((page, index))` avant l'envoi, résolu dans `DocumentChanged` comme `edit.reselect` (`app.rs:446-451`) ; les indices restent stables sauf suppression. Flèches : `nudge_selected` (`editor.rs:483`) ; `Suppr` : `delete_selected_object` → `EditOp::DeleteAnnotation` si `replies == 0`, sinon `DeleteAnnotationThread`.
* **Barre flottante** (`object_bar`, `editor.rs:609`) : visible pour tous les outils de dessin/sélection d'annotation. Ajouts : `DragValue` « Opacité » (10–100 %), case « Flèche » (outil Ligne), boutons Répondre / Supprimer (annotation sélectionnée) ; « Appliquer à l'annotation » n'est actif que si `can_restyle` et envoie `EditOp::StyleAnnotation`. Les couleurs de la barre se synchronisent sur l'annotation sélectionnée (comme `on_object_selected`, `editor.rs:450`).
* **Palette** (`tool_palette`, `annotate.rs:108`) : 18 boutons × 34 px dépassent une petite fenêtre → envelopper la colonne dans `egui::ScrollArea::vertical().max_height(…)` avec des séparateurs entre groupes (texte | annotations | sélection | contenu).
* **Auteur** : champ texte dans l'en-tête du panneau Commentaires (clés `comments.author*`), valeur dans `Settings.author` (persistée via `persist_settings`, `app.rs:319`) ; vide → `/T` omis.
* **Panneau Commentaires** (`comments_panel`, `annotate.rs:303`) : construire les fils à partir de `adv.details` (si absents : liste rapide à plat, comportement actuel) : cartes pour les annotations sans `parent` et qui ne sont pas des annotations d'état ; sous chaque carte : statut courant (puce) + réponses (cartes réduites : auteur, date `M` formatée, texte) ; boutons **Répondre** (ouvre `ReplyPrompt` → `EditOp::Reply`), **Statut** (`ComboBox` → `EditOp::SetReviewStatus`), **Supprimer** / **Supprimer le fil**. Clic sur une carte : `tool = SelectAnnot` et sélection. Masquer les `Popup`.

### 5.5 Comportement avec onglets, annulation, enregistrement

* L'état vit dans `AnnotateState` (`Default`, échangé par `take_tab`/`put_tab`, `tabs.rs:52-100`) : brouillons, sélection et caches sont donc par document. Les préférences de style vivent dans `EditState` (globales).
* Annuler/Rétablir : instantanés complets existants (`State::undo`, `worker.rs:359-413`) ; une opération = un niveau. Après `Undo`, `DocumentChanged` remet à zéro `details_page`/`links_page`/sélection d'annotation.
* Le drapeau « modifié » (`dirty`) est posé par `DocumentChanged` (`app.rs:456`) ; `Enregistrer` utilise `save_as` (écriture atomique, `pages.rs:276`).
* Les annotations créées par lopdf n'ont pas besoin de rendu préalable pour être enregistrées correctement (AP explicites) ; seules les recolorations de surlignages exigent `ensure_appearances` (§3.3).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Sortie JSON, messages en anglais, coordonnées `--rect left,bottom,right,top` en **points de page affichée** (identiques aux points PDF pour une page non tournée), pages **1-based**, indices d'annotation **0-based** (ceux de `annotations`).

| Sous-commande | Arguments | Sortie JSON |
|---|---|---|
| `annotations <file> [--page N] [--password P]` | toutes les pages par défaut | `{"pages":[{"page":1,"annotations":[{"index":0,"subtype":"Square","rect":[100,100,200,150],"contents":null,"author":"Alice","modified":"D:20261014…Z","name":"SuniPDF-…","color":"#dc2626","fill":null,"width":2.0,"opacity":1.0,"parent":null,"status":null,"link":null,"ours":true,"replies":0}]}]}` (utilise `annotation_details` ; `link` = `{"page":2}` ou `{"uri":"https://…"}`) |
| `annotate <file> --page N --kind ink\|square\|circle\|line\|arrow\|freetext\|link --out OUT [--rect L,B,R,T] [--stroke "x,y x,y …"]… [--text T] [--size 12] [--color RRGGBB] [--fill RRGGBB] [--width 2] [--opacity 1] [--goto-page M \| --url U] [--author A] [--password P]` | `--stroke` répétable (ink) ; `link` exige `--goto-page` ou `--url` (contrôle `is_safe_external_uri` → erreur `other`, `"unsafe uri"`) | `{"out":"…","page":1,"kind":"ink","annotation_count":3}` |
| `annotation-delete <file> --page N --index I [--thread] --out OUT` | | `{"out":"…","deleted":3}` (nombre d'annotations retirées) |
| `reply <file> --page N --index I --text T [--author A] --out OUT` | | `{"out":"…","index":7}` |
| `review-state <file> --page N --index I --state none\|accepted\|rejected\|cancelled\|completed [--author A] --out OUT` | | `{"out":"…","state":"accepted"}` |

Sur document chiffré : erreur `{"error":"security_restricted",…}` (code de sortie 1). Écriture : `doc.save_as(&out)` (atomique). Le décalage/redimensionnement/style n'ont pas de commande CLI (outils d'interface ; couverts par les tests moteur). Ajouter les 5 commandes à la liste de la doc CLI (`README`/`USER_GUIDE`).

---

## 7. i18n (nouvelles clés, **fr-FR** puis **en-US**)

Ajouter dans `assets/i18n/fr-FR.json` **et** `en-US.json` (`scripts\check-i18n.ps1` exige des clés identiques ; tests `cargo test -p sunipdf-i18n -p sunipdf-desktop`). Les clés `shortcut.*` ont la même valeur dans les deux langues.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.edit.tool_select_annot` | Sélectionner une annotation | Select an annotation |
| `menu.edit.tool_free_text` | Zone de texte libre | Free text box |
| `menu.edit.tool_ink` | Dessin à main levée | Freehand drawing |
| `menu.edit.tool_annot_rect` | Rectangle (annotation) | Rectangle (annotation) |
| `menu.edit.tool_annot_ellipse` | Ellipse (annotation) | Ellipse (annotation) |
| `menu.edit.tool_annot_line` | Ligne ou flèche (annotation) | Line or arrow (annotation) |
| `menu.edit.tool_link` | Lien | Link |
| `shortcut.tool_select_annot` | A | A |
| `shortcut.tool_free_text` | F | F |
| `shortcut.tool_ink` | D | D |
| `status.annot_selected` | {kind} sélectionné : glisser pour déplacer, poignées pour redimensionner, flèches pour décaler, Suppr pour supprimer | {kind} selected: drag to move, handles to resize, arrow keys to nudge, Delete to remove |
| `status.link_unsupported` | Ce type de lien n'est pas pris en charge. | This type of link is not supported. |
| `status.annot_locked` | Cette annotation est verrouillée. | This annotation is locked. |
| `bar.opacity` | Opacité (%) | Opacity (%) |
| `bar.arrow` | Flèche | Arrow |
| `bar.annot_apply` | Appliquer à l'annotation | Apply to annotation |
| `bar.annot_apply_hint` | Applique ces couleurs à l'annotation sélectionnée ({kind}) | Applies these colors to the selected annotation ({kind}) |
| `bar.reply` | Répondre | Reply |
| `dialog.free_text.title` | Zone de texte libre | Free text box |
| `dialog.free_text.title_edit` | Modifier la zone de texte | Edit text box |
| `dialog.free_text.subtitle` | Le texte devient une annotation modifiable. | The text becomes an editable annotation. |
| `dialog.free_text.placeholder` | Saisissez votre texte… | Type your text… |
| `dialog.free_text.background` | Fond | Background |
| `dialog.free_text.border` | Bordure (pt) | Border (pt) |
| `dialog.free_text.ok` | Ajouter | Add |
| `dialog.free_text.ok_edit` | Enregistrer | Save |
| `dialog.free_text.latin_note` | Police Helvetica : caractères latins uniquement dans l'apparence. | Helvetica font: Latin characters only in the appearance. |
| `dialog.link.title` | Ajouter un lien | Add a link |
| `dialog.link.subtitle` | Rend la zone cliquable dans le document. | Makes the area clickable in the document. |
| `dialog.link.kind_page` | Aller à une page | Go to a page |
| `dialog.link.kind_url` | Ouvrir une adresse web | Open a web address |
| `dialog.link.page` | Page | Page |
| `dialog.link.url` | Adresse (http, https ou mailto) | Address (http, https or mailto) |
| `dialog.link.url_hint` | https://exemple.org | https://example.org |
| `dialog.link.invalid_url` | Adresse refusée : seuls http, https et mailto sont acceptés. | Address rejected: only http, https and mailto are accepted. |
| `dialog.link.ok` | Ajouter le lien | Add link |
| `dialog.url.title` | Ouvrir un lien externe ? | Open an external link? |
| `dialog.url.subtitle` | SuniPDF fonctionne hors ligne : rien n'est ouvert sans votre accord. | SuniPDF works offline: nothing is opened without your consent. |
| `dialog.url.body` | Ce document demande d'ouvrir l'adresse suivante dans votre navigateur. N'ouvrez que des liens de confiance. | This document asks to open the following address in your browser. Only open links you trust. |
| `dialog.url.open` | Ouvrir dans le navigateur | Open in browser |
| `dialog.url.copy` | Copier l'adresse | Copy address |
| `dialog.reply.title` | Répondre | Reply |
| `dialog.reply.subtitle` | Votre réponse sera ajoutée au fil de discussion. | Your reply will be added to the discussion thread. |
| `dialog.reply.placeholder` | Votre réponse… | Your reply… |
| `dialog.reply.ok` | Répondre | Reply |
| `comments.author` | Auteur | Author |
| `comments.author_hint` | Nom enregistré dans vos annotations | Name stored in your annotations |
| `comments.reply` | Répondre | Reply |
| `comments.replies` | Réponses ({n}) | Replies ({n}) |
| `comments.status` | Statut | Status |
| `comments.status.none` | Aucun | None |
| `comments.status.accepted` | Accepté | Accepted |
| `comments.status.rejected` | Refusé | Rejected |
| `comments.status.cancelled` | Annulé | Cancelled |
| `comments.status.completed` | Terminé | Completed |
| `comments.delete_thread` | Supprimer le fil | Delete thread |
| `comments.kind.freetext` | Texte libre | Free text |
| `comments.kind.ink` | Dessin | Drawing |
| `comments.kind.square` | Rectangle | Rectangle |
| `comments.kind.circle` | Ellipse | Ellipse |
| `comments.kind.line` | Ligne | Line |
| `comments.kind.squiggly` | Ondulé | Squiggly |
| `comments.kind.stamp` | Tampon | Stamp |

Total : 60 clés × 2 langues. Le libellé `comments.empty` existant reste valide. Les erreurs moteur réutilisent `error.security` (document chiffré/verrouillé) et `error.unknown`.
Étendre le test `ui_keys_used_by_app_exist` (`app.rs:1964`) pour inclure `annot_adv.rs`, `annotate.rs`, `editor.rs` dans la concaténation `include_str!` (vérifie automatiquement les nouvelles clés littérales).

---

## 8. Tests

### 8.1 Fixtures (à ajouter à `scripts/make_fixtures.py`, régénération : `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`)

* `annotated.pdf` — page 1 (612×792, texte « SuniPDF page 1 » en (72,700), comme `two-pages.pdf`) + page 2 (792×612). 12 annotations sur la page 1, **dans cet ordre** (indices 0…11) :
  0 `Square` (style Acrobat, avec `/AP`, `/C` rouge, `/T (Alice)`, `/Contents (Cadre rouge)`, Rect [100 500 200 560], sans `/NM` SuniPDF) · 1 `Ink` (`/InkList [[110 400 150 450 190 400]]`, avec `/AP`) · 2 `FreeText` (`/DA`, `/AP`, Rect [300 400 460 440]) ·
  3 `Highlight` jaune sur le titre (`/QuadPoints`, **sans** `/AP`) · 4 `Text` note (`/Contents (Note parent)`, `/T (Alice)`, `/Popup 11 0 R`, sans `/AP`) · 5 `Text` réponse (`/IRT` vers 4, `/RT /R`, `/T (Bob)`, `/Contents` « Réponse de Bob » en UTF-16BE) ·
  6 `Text` état (`/IRT` vers 4, `/StateModel /Review`, `/State /Accepted`) · 7 `Link` `/A << /S /URI /URI (https://example.com) >>` Rect [72 700 300 730] · 8 `Square` intercalé (régression « doublons de `links()` ») ·
  9 `Link` `/Dest [page2 /Fit]` Rect [100 300 200 330] · 10 `Link` `/A << /S /Launch /F (calc.exe) >>` (doit rester `Unsupported`) · 11 `Popup` (`/Parent` = note 4).
* `rotated-crop.pdf` — page 1 : `/Rotate 90`, `/MediaBox [0 0 612 792]`, `/CropBox [50 60 562 732]`, un carré noir plein 40×40 pt en (300,400) ; page 2 : `/Rotate 270`, sans CropBox.
* Réutiliser `two-pages.pdf` et `encrypted-rc4.pdf` (mot de passe `secret`).

### 8.2 Tests unitaires purs (dans les modules, sans PDFium)

* `geometry.rs` : tableau de 8 cas (rotation 0/90/180/270 × CropBox absente / [50 60 562 732]) pour le rectangle utilisateur (100,100,200,150) → rectangle affiché **mesuré sur PDFium 7881** [F8] (origine en bas à gauche, hauteur affichée Hd) :
  `R0` (100,100,200,150) · `R0+crop` (50,40,150,90) · `R90` (100,412,150,512) · `R90+crop` (40,362,90,462) · `R180` (412,642,512,692) · `R180+crop` (362,582,462,632) · `R270` (642,100,692,200) · `R270+crop` (582,50,632,150) ;
  aller-retour `user_to_display ∘ display_to_user` = identité ; `display_size`.
* `appearance.rs` : somme des largeurs de la chaîne ASCII imprimable = 600,7 pt à 12 pt (±0,01) ; `wrap_text` (largeur serrée, mot plus long que la ligne coupé, `\n` conservés, aucune ligne vide ajoutée) ; `winansi_literal("é(")` → `\351\(` ; `fmt_num` (`1.0`→`1`, `0.123456`→`0.123`) ;
  `simplify_polyline` (points alignés → 2 points ; courbe → ε respecté) ; contenus : `square_ap` contient `re` et `B`/`S`/`f` selon fond/épaisseur, `circle_ap` contient 4 `c`, `line_ap` flèche → 2 `l` de plus.
* `annot_write.rs` : `pdf_date_now` à partir d'`UNIX_EPOCH` → `D:19700101000000Z` et 1 000 000 000 s → `D:20010909014640Z` ; `SuniPDF-…` : 24 caractères hexadécimaux.
* `links.rs` : `is_safe_external_uri` accepte `https://a.b/c?d=1`, `http://x`, `mailto:a@b.c`, `HTTPS://X.ORG` ; refuse `file:///C:/x`, `javascript:alert(1)`, `ms-msdt:id`, `C:\Windows\calc.exe`, `\\srv\share`, `https:///x`, `https://a b`, `https://a\tb`, vide, 2049 caractères.

### 8.3 Tests d'intégration `crates/sunipdf-core/tests/engine.rs` (moteur sérialisé `engine()`, vérification par rendu ou relecture)

1. `annotation_geometry_rotated_and_cropped_pages` — `rotated-crop.pdf` : `page_geometry(0)` = `R90` + crop (50,60,562,732) ; `render_page` a la taille `display_size` ; ajouter un `Square` rouge plein (rect affiché (40,362,90,462), fond rouge) → pixels rouges rendus dans le rectangle **device** attendu (40..90, 50..150, tolérance 1 px) ; idem page 2 (`R270`).
2. `ink_annotation_round_trip` — `two-pages.pdf` : deux traits bleus (épaisseur 4) → `annotations(0)` contient un `Ink` de rect ≈ boîte des points ± 2,5 ; > 300 pixels bleus dans la boîte, **0** pixel bleu hors de la boîte + 3 px ; `lopdf` relit `/InkList` (2 tableaux) ; `save_as` → `engine.open` → même nombre de pixels bleus (±2 %), `annotation_details(0)[0].ours == true`, `author == Some("Alice")` ; moins de 2 points → `Err`.
3. `shape_annotations_render` — Square avec fond jaune opacité 0,5 : pixel central ≈ jaune sur blanc (bleu 100–160) ; Circle sans fond : centre blanc, anneau coloré ; Line vs Arrow : l'arrow a plus de pixels colorés que la ligne seule, la pointe est près de l'extrémité ; `Line` de longueur 0 → `Err`.
4. `free_text_wraps_and_grows` — 120 caractères, 12 pt, boîte de 150 pt de large et 20 pt de haut : `details.rect` plus haute que 20 (grandit vers le bas, bord supérieur inchangé) ; ≥ 3 bandes de lignes de pixels sombres ; aucun pixel sombre hors du rectangle (+1 px) ; `contents` identique à l'entrée (accents conservés) ; texte avec « ✓ » : apparence OK (`?`), contenu inchangé ; texte vide → `Err`.
5. `link_annotations_and_safety` — création `Page(1)` et `Uri("https://example.org")` entre deux `Square` : `page_links(0)` renvoie **exactement 2** entrées (régression doublons) avec cibles correctes et rects ; `Page(9)` → `PageOutOfRange(9)` ; `Uri("file:///C:/Windows/System32/calc.exe")` et `javascript:` → `Err` ; document relu après `save_as` : mêmes liens.
6. `annotated_fixture_details` — `annotated.pdf` : `annotation_details(0)` : sous-types dans l'ordre ; `parent` de la réponse et de l'état = 4 ; statut `Accepted` sur l'annotation d'état ; liens (indices 7, 9, 10) `Uri`, `Page(1)`, `Unsupported` ; `ours == false` partout ; `replies` de la note = 2 ; `Popup` présent.
7. `edit_existing_annotations` — sur `annotated.pdf` : (a) `move_annotation` du `Square` (+30, −20) : `Rect` translaté, pixels rouges déplacés de (30, 20) ±1 ; `InkList` de l'encre déplacée après `move` de l'encre ; les autres annotations inchangées (comparaison des `Debug` de leurs dictionnaires via lopdf avant/après) ; (b) `resize_annotation` sur annotation étrangère → `Err` ; (c) `set_annotation_style` du surlignage en vert : rendu jaune → vert, `/AP` régénéré après `ensure_appearances` (lopdf relit `/AP`), nombre d'annotations constant ; (d) `set_annotation_contents` ; (e) `delete_annotation_thread(note)` retire la note, la réponse, l'état et le `Popup` (−4) et laisse tout le reste ; (f) annotation `ours` créée puis `resize_annotation` : rendu dans le nouveau rectangle (±1 px) et `AP` régénéré ; `style` recolore le rendu.
8. `replies_and_status_are_invisible_and_threaded` — `add_note` (PDFium) puis `add_reply` ×2 et `set_review_status(Accepted)` puis `Rejected` : `details` : `parent`, `status` de la dernière annotation d'état ; **rendu identique pixel à pixel** avant/après les réponses (aucune icône) ; `delete_annotation_thread` supprime le fil.
9. `lopdf_annotation_edits_refuse_encrypted_documents` — `encrypted-rc4.pdf` ouvert avec `secret` : `add_ink_annotation`, `add_shape_annotation`, `add_free_text_annotation`, `add_link_annotation`, `move_annotation` → `Err(SecurityRestricted)` ; `snapshot()` contient toujours `/Encrypt` ; `add_note` (PDFium) fonctionne encore.
10. `annotation_snapshot_undo` — `snapshot` → ajout d'un `Ink` + `Reply` → `restore(snapshot)` → `annotations(0)` revient au nombre initial.
11. `annotations_list_aligned_with_pdf_array` — page avec entrée `null` au milieu de `/Annots` (fixture ou octets modifiés) : `annotations()` conserve les indices (entrée `Unknown`), `annotation_details()` aligné.
12. Tests CLI : `cargo test -p sunipdf-cli` (parsing `--rect`, `--stroke`) + exécution manuelle des 5 commandes sur les fixtures avec relecture JSON.
13. Vérification qpdf quand l'outil sera présent en CI (`docs/KNOWN_ISSUES.md` A4, SUNI-025) : `qpdf --check` sur `annotated.pdf` après chaque opération. **Non exécuté ici** : qpdf n'est pas installé sur ce poste (à confirmer avant de l'annoncer).

### 8.4 Scénarios de capture (`scripts\dev-screenshot.ps1`, coordonnées de fenêtre indicatives, zoom « Largeur » ; faire d'abord une capture sans `-Drag` pour caler les positions)

```powershell
# 1. Encre
.\scripts\dev-screenshot.ps1 -Out ink.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands 'tool.ink' -Drag '420,260,470,330;470,330,520,250;520,250,570,330'
# 2. Rectangle, ellipse puis flèche (une exécution par outil : un seul outil actif par lancement)
.\scripts\dev-screenshot.ps1 -Out annot-rect.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands 'tool.annot_rect' -Drag '400,280,560,360'
.\scripts\dev-screenshot.ps1 -Out annot-arrow.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands 'tool.annot_line' -Drag '400,420,560,330'
# 3. Texte libre : clic puis dialogue ouvert (capture du dialogue)
.\scripts\dev-screenshot.ps1 -Out freetext.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Commands 'tool.free_text' -Drag '400,300,420,310'
# 4. Sélection + déplacement d'une annotation existante (1er geste = sélection, 2e = déplacement)
.\scripts\dev-screenshot.ps1 -Out annot-move.png -Pdf crates\sunipdf-core\tests\fixtures\annotated.pdf -Commands 'tool.select_annot' -Drag '470,300,473,303;470,300,540,360'
# 5. Panneau des commentaires avec fil (note + réponse + statut)
.\scripts\dev-screenshot.ps1 -Out comments-thread.png -Pdf crates\sunipdf-core\tests\fixtures\annotated.pdf -Commands 'view.comments'
# 6. Clic sur un lien externe : boîte de confirmation (outil par défaut)
.\scripts\dev-screenshot.ps1 -Out link-confirm.png -Pdf crates\sunipdf-core\tests\fixtures\annotated.pdf -Drag '430,215,432,216'
# 7. Raccourci clavier (D) puis capture de la barre d'outils
.\scripts\dev-screenshot.ps1 -Out ink-key.png -Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -KeysAfter 'd'
```

Contrôles visuels à noter dans le rapport : tracé lissé sans crénelage excessif, poignées alignées, curseur main sur lien, dialogues cohérents avec `dialog.rs`, thème sombre (relancer 1 et 5 avec le thème sombre), absence d'icône de note pour les réponses, icônes Fluent lisibles (sinon ajuster les points de code).

---

## 9. Nouvelles dépendances

**Aucune.** `lopdf` 0.45 (MIT), `getrandom` 0.3, `image`, `pdfium-render` 0.9.4 et `egui`/`eframe` 0.36 (via `ctx.open_url` → `webbrowser` déjà présent dans `Cargo.lock`) suffisent. `chrono` (déjà dans l'arbre via `lopdf`/`pdfium-render`) n'est volontairement pas ajouté en dépendance directe : 12 lignes de calendrier civil suffisent. Aucune ligne à ajouter à `docs/THIRD_PARTY_NOTICES.md`.

---

## 10. Risques, pièges connus, ordre d'implémentation, estimation

### 10.1 Risques et pièges (tous vérifiés ou sourcés, cf. annexe A)

1. **`pdfium-render` ne suffit pas** : pas de `create_circle_annotation`, pas de ligne, pas de trait d'encre, pas d'accès aux clés `/DA` `/BS` `/IRT` ; handles bruts inaccessibles → tout passe par lopdf **[F1][F3]**.
2. **Repli de couleur dangereux** de `pdfium-render` (transtypage annotation → objet de page) dès qu'un `/AP` existe **[F4]** : ne jamais lire/écrire les couleurs d'annotation par cette API.
3. **Apparence FreeText PDFium inutilisable** (une ligne, pas de `/Resources`, `/C` rend un fond noir) → `/AP` écrit par SuniPDF **[F5]**.
4. **Documents chiffrés** : `security::load` retire `/Encrypt` ; refus typé dans ce plan. **Défaut latent voisin** : `edit.rs:402` (`merge_content_streams`, utilisé par « Mettre à l'arrière-plan », `reorder_object` `edit.rs:424`) déchiffre silencieusement le document ouvert, qui sera ensuite enregistré **en clair** **[F11]**. Correctif trivial recommandé dans la même livraison : réutiliser `load_for_edit` dans cette fonction (+ test sur `encrypted-rc4.pdf`).
5. **Coût d'un aller-retour complet** (instantané PDFium + lopdf + rechargement) à chaque opération et à chaque trait d'encre : correct pour des PDF courants, lent pour des documents de dizaines de Mo (secondes). Mitigation prévue : le regroupement de traits est laissé à une évolution (`ink_pending` + validation à l'inactivité), non fait ici pour éviter la perte de données à l'enregistrement/fermeture.
6. **Thread moteur unique** : `annotation_details` bloque les rendus pendant son exécution → n'est demandé que si la page a des annotations « utiles » et si le panneau/outil de sélection est actif.
7. **Désalignement d'indices** : `iter()` de `pdfium-render` s'arrête à la première entrée invalide ; `lopdf::get_page_annotations` saute les entrées invalides ; indexer les entrées brutes (test 11).
8. **Liens** : `links().iter()` duplique ; destination `/XYZ` ignorée (page seulement) ; destinations nommées non résolues ; actions Launch/JavaScript jamais exécutées.
9. **Sécurité des URL** : `webbrowser::open` n'a pas de filtre par défaut ; liste d'autorisation + confirmation obligatoires ; afficher l'adresse **complète** (hameçonnage par `user@hôte`, homographes : l'utilisateur voit exactement ce qui sera ouvert) ; jamais de réseau depuis SuniPDF lui-même.
10. **Rotation/CropBox** : corrigés pour les nouveaux outils seulement. Les outils existants (sélection de texte, objets, formes, tampons, `add_note`) supposent une page non tournée/non rognée → à consigner dans `docs/KNOWN_ISSUES.md` (A11) et à traiter par une tâche séparée qui généraliserait `PageGeometry` à `annotate::to_pdf` et aux `to_screen` locaux.
11. **PDFium écrit `PDFIUM_HasGeneratedAP`** dans les annotations dont il génère l'apparence (observé) : inoffensif ; les annotations créées par SuniPDF ne l'ont pas.
12. **Réponses** : compatibilité Acrobat/Foxit non vérifiée ; AP vide + `F 4` choisis pour éviter une icône par réponse dans PDFium. Si un lecteur affiche mal, prévoir un repli (`F 2` Hidden).
13. **Texte libre** : polices standard seulement ; la table de largeurs ne couvre exactement que l'ASCII (marge de 2 %) ; texte long très étroit → mots coupés.
14. **Encre** : jusqu'à 4 000 points/trait (plafond) ; épaisseur > 50 pt refusée ; un seul trait par annotation.
15. **Signatures numériques** : toute réécriture les invalide (comme les autres éditions de SuniPDF) — aucune signature n'est gérée à ce jour.
16. **Conflit de fusion** avec le plan de vue continue : `document_area` (`app.rs:1058-1233`) sera remanié ; isoler la logique dans `annot_adv.rs`, n'ajouter à `app.rs` que des appels d'une ligne.
17. **Tests** : PDFium n'est pas thread-safe ; utiliser le moteur sérialisé `engine()` (`tests/engine.rs:34`) ; ne pas comparer `/NM` (aléatoire).
18. **Mémoire du poste (14 Go)** : lancer `cargo test --workspace -j 2` uniquement ; une instance de test à la fois.

### 10.2 Ordre d'implémentation (étapes compilables, chacune se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p sunipdf-core -j 2`)

| Étape | Contenu | Taille |
|---|---|---|
| **E0** | `geometry.rs` + tests purs ; `annot.rs` (rect affiché, `author`, `modified`, itération par `get(i)`, `annotation_at`) ; `security::lopdf_err` en `pub(crate)` ; **correctif `merge_content_streams`** (garde chiffrement) | S |
| **E1** | `appearance.rs` complet + tests purs (aucun PDFium) | M |
| **E2** | `annot_write.rs` création (`load_for_edit`, `edit_pdf`, `annots_mut`, `push_annotation`, `add_ink/shape/free_text/link`), `links.rs` (`page_links`, `is_safe_external_uri`), `pdf_date_now`, `/NM` ; fixtures `rotated-crop.pdf` ; tests d'intégration 1–5, 9, 10 | L |
| **E3** | Worker (EditOp création, `Links`) + commandes/`Tool`/icônes/i18n (clés création) + UI de création : encre (aperçu), formes, texte libre (dialogue), lien (dialogue), barre (opacité, flèche), palette défilante, `Settings.author` ; captures 1–3 et 7 | M |
| **E4** | Clic sur les liens : cache `Links`, curseur main, saut de page, confirmation d'URL, `open_url` ; CLI `annotate` + `annotations` (liste PDFium/détails) ; capture 6 | S |
| **E5** | `annotation_details`, `move/resize/style/contents/delete_thread` ; fixture `annotated.pdf` ; tests 6, 7, 11 ; UI `Tool::SelectAnnot` (généralisation de la sélection d'`editor.rs`, poignées, flèches, Suppr, barre) ; capture 4 | L |
| **E6** | `add_reply`, `set_review_status` + panneau Commentaires en fils, champ Auteur, `Reply`/`Status` dans l'UI ; CLI `reply`, `review-state`, `annotation-delete` ; test 8 ; capture 5 | M |
| **E7** | Documentation (§11), `CHANGELOG`, version 0.4.0, exécution complète `cargo test --workspace -j 2`, `scripts\check-i18n.ps1`, `cargo fmt --all -- --check`, rapport | S |

Recommandation de livraison en **deux paquets** : *021b-1* = E0–E4 (création d'annotations, liens cliquables) ; *021b-2* = E5–E7 (édition des annotations existantes, fils, statuts). Chaque paquet laisse l'application cohérente (pas de bouton factice : les outils d'un paquet ne sont enregistrés qu'avec leur code complet).

### 10.3 Estimation

**XL** au total (moteur ≈ 1 500 lignes + 600 lignes de tests, UI ≈ 900 lignes, CLI ≈ 250 lignes, ≈ 60 clés i18n × 2, 2 fixtures). Par paquet : *021b-1* = L, *021b-2* = L.

### 10.4 Questions ouvertes (à trancher avant ou pendant E3)

1. Accepte-t-on le **refus** des écritures lopdf sur documents chiffrés, ou faut-il investir dans un ré-chiffrement fidèle (même version/permissions) ?
2. Auteur par défaut : vide (choix du plan, aucune fuite du nom Windows) ou nom de session Windows proposé au premier usage ?
3. Souhaite-t-on regrouper les traits d'encre (une annotation pour N traits, validée à l'inactivité) malgré le risque de perte à l'enregistrement ? (le plan livre « un trait = une annotation »)
4. Ouvrir une tâche séparée « géométrie de page pour les outils existants » (rotation/CropBox pour sélection de texte, objets, formes, tampons) ?
5. Réponses/statuts : accepte-t-on de les livrer sans test de compatibilité Acrobat (non disponible ici) ?
6. La palette à 18 outils doit-elle devenir contextuelle (n'afficher que l'onglet de la barre d'outils actif) ?

---

## 11. Documentation à mettre à jour

* `docs/FEATURE_MATRIX.md` (section B) : ligne « Notes, surlignage… formes, encre, zones de texte » → **Fait** pour encre, texte libre, rectangle/ellipse/ligne/flèche en annotations ; ligne « Fils de réponses, statut » → **Partiel (Fait dans SuniPDF, interop Acrobat non vérifiée)** ; ligne « Liens » → **Fait** (création, clic interne, URL externe confirmée) ; mentionner les limites (§2).
* `docs/PROJECT_STATUS.md` : entrée SUNI-021b avec commandes exécutées et résultats **réels** (jamais avant exécution), nombre de tests, captures inspectées ; « Non réalisé » : retirer « dessin libre (encre) ».
* `docs/NEXT_TASKS.md` : retirer/réduire SUNI-021b ; ajouter « géométrie de page pour les outils existants », « regroupement des traits d'encre », « ré-chiffrement après édition lopdf », « vérification qpdf des annotations ».
* `docs/ROADMAP.md` (lignes 22-38 : « liens ; commentaires avec fils ; annotations avancées ») : cocher ce qui est fait.
* `docs/DECISIONS.md` : **D021** — annotations avancées écrites par lopdf avec `/AP` explicite (PDFium = rendu, lecture, instantanés) car `pdfium-render` 0.9.4 ne couvre pas Circle/Line/Ink/DA/IRT et expose un repli de couleur non sûr ; **D022** — liens externes : liste d'autorisation `http/https/mailto` + confirmation explicite + aucune ouverture automatique (hors ligne d'abord) ; **D023** — API d'annotation en coordonnées de page affichée (rotation + CropBox) ; **D024** — refus des écritures lopdf sur documents chiffrés tant que le ré-chiffrement n'existe pas.
* `docs/KNOWN_ISSUES.md` : A11 rotation/CropBox des outils existants ; A12 écritures lopdf refusées sur documents chiffrés (+ latence de `merge_content_streams` corrigée) ; A13 texte libre Helvetica/WinAnsi ; A14 réponses/statuts non testés avec Acrobat ; A15 un trait d'encre = une annotation, coût par opération sur gros fichiers ; A16 `pdfium-render` : `links().iter()` duplique, `stroke_color()` non sûr.
* `docs/ARCHITECTURE.md` : liste des modules du moteur (`geometry`, `appearance`, `annot_write`, `links`) et paragraphe « annotations : lopdf écrit, PDFium rend ».
* `docs/I18N.md` : espace de noms `comments.*`/`dialog.free_text.*`/`dialog.link.*`/`dialog.url.*`/`dialog.reply.*`.
* `docs/USER_GUIDE.fr.md` / `docs/USER_GUIDE.en.md` : nouvelle section « Annoter » (outils, raccourcis `A`/`D`/`F`, liens, fils de réponses, limites).
* `CHANGELOG.md` : 0.4.0.
* `README.md` / aide CLI : 5 nouvelles sous-commandes.
* `docs/THIRD_PARTY_NOTICES.md` : aucun changement (pas de dépendance ni de ressource nouvelle).

---

## Annexe A — Faits vérifiés (sources et expériences)

Légende : « src » = lecture des sources locales (chemins sous `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`) ; « exp » = expérience jetable avec Python 3.11 + `ctypes` sur
`third_party/pdfium/win-x64/bin/pdfium.dll` (build 7881), scripts passés par l'entrée standard, **aucun fichier du dépôt créé ni modifié** (les tests d'intégration du §8 codifient les mêmes constats ; l'implémenteur peut refaire chaque expérience en quelques minutes).

| Réf. | Fait | Preuve |
|---|---|---|
| F1 | `PdfPageAnnotations` sait créer FreeText, Highlight, Ink, Link (URI uniquement), Popup, Square, Squiggly, Stamp, Strikeout, Text, Underline — **pas Circle, pas Line**. Les constructeurs appellent `FPDFPage_CreateAnnot` (`annotations.rs:149-373`, `bindings.rs:3581`). | src |
| F2 | PDFium 7881 : `FPDFAnnot_IsSupportedSubtype` vaut 1 pour Text, Link, FreeText, Square, Circle, Highlight, Underline, Squiggly, StrikeOut, Stamp, Ink, Popup, FileAttachment et **0 pour Line, Polygon, Polyline, Caret** ; `FPDFPage_CreateAnnot(Line)` renvoie NULL ; Link, FreeText, Square, Circle, Ink se créent. | exp |
| F3 | Ink : `PdfPageInkAnnotation` n'expose que `objects_mut()` (`annotation/ink.rs:27-50`) ; seuls Ink et Stamp acceptent des objets de page (`annotation/objects.rs:27-30`), ce qui produit une apparence sans `/InkList`. `FPDFAnnot_AddInkStroke` n'existe qu'en binding brut (`bindings.rs:3690`) et le handle brut est inaccessible (`annotation.rs:11` module `private` en `pub(crate)`, `private.rs:131` `fn handle`). `set_link` n'écrit que l'URI (`link.rs:76`). Aucun accès à `/DA`, `/BS` (`FPDFAnnot_SetBorder` brut, `bindings.rs:3990`), `/IRT` (`FPDFAnnot_GetLinkedAnnot` brut, `:4288`). | src |
| F4 | L'apparence est générée **au premier rendu** : un document sauvegardé juste après création n'a pas de `/AP` ; après un rendu de 16×16 px, `/AP` + `/PDFIUM_HasGeneratedAP true` sont présents. Une fois l'`/AP` présent, `FPDFAnnot_SetColor` renvoie 0. `pdfium-render` retombe alors sur `FPDFPageObj_SetStrokeColor/…GetFillColor(self.handle() as FPDF_PAGEOBJECT)` (`private.rs:410,442-460,494,526-540`). | exp + src |
| F5 | FreeText généré par PDFium : une seule ligne non renvoyée, police `/Helv` sans `/Resources`, `/DA` utilisée pour le cadre (`1 0 0 rg … f*`), et `FPDFAnnot_SetColor(…0,0,0…)` donne un **fond noir** (la clé `/C` d'un FreeText est le fond). | exp |
| F6 | PDFium rend une annotation `Line` portant un `/AP` explicite (boîte rendue 99..301 × 589..692 pour une ligne de (100,100) à (300,200) avec flèche) et la rapporte comme `Line` (`annotation.rs:434-435`, `unsupported.rs` `get_type`). | exp + src |
| F7 | Des `/AP` explicites (Form XObject, `BBox [0 0 w h]`, `Matrix` identité) pour Square, Circle, FreeText (police `/Helv` Helvetica WinAnsi déclarée dans `/Resources` du flux) et Line sont rendus à la position de `/Rect`, y compris sur pages tournées/rognées. | exp |
| F8 | Conversion affichage ↔ utilisateur : les formules du §3.1 reproduisent exactement la position rendue pour `/Rotate` 0/90/180/270 avec et sans `/CropBox [50 60 562 732]` (8 combinaisons, erreur 0 px) ; tailles de page : 612×792, 512×672, 792×612, 672×512. | exp |
| F9 | `links().get(i)` passe `i` comme indice d'annotation à `FPDFLink_Enumerate` (`links.rs:116-129`) : avec `[Square, Link, Square, Link]`, `get(0..3)` renvoie 4 résultats pour 2 liens. `/Dest [page /Fit]` et `/A << /S /GoTo /D … >>` donnent tous deux la page 1 (0-based) via `FPDFLink_GetDest`/`FPDFDest_GetDestPageIndex` ; `/A << /S /URI >>` donne l'action de type 3 et `FPDFAction_GetURIPath`. `PdfPageIndex = c_int` (`pdfium-render/src/pdf/document/pages.rs:26`). | exp + src |
| F10 | Supprimer `/AP` et `/PDFIUM_HasGeneratedAP` d'un surlignage, changer `/C`, puis recharger : PDFium régénère l'apparence au rendu (même 16×16 px) et la mémorise à l'enregistrement (jaune 5000 px → vert 5000 px). | exp |
| F11 | Le snapshot PDFium d'un document chiffré contient `/Encrypt` (`encrypted-rc4.pdf` ouvert avec `secret`). `security::load` supprime `/Encrypt` après déchiffrement (`security.rs:78-95`) ; `security::save` écrit en clair (`:96`). `merge_content_streams` (`edit.rs:402-422`) enchaîne `load` → `save` → `restore` : le document ouvert devient non chiffré. | exp + src |
| F12 | `FPDFPage_GetAnnot(i)` lit la i-ème entrée de `/Annots` ; `PdfPageAnnotationsIterator::next` s'arrête à la première erreur (`annotations.rs:619-628`) ; `lopdf::get_page_annotations` saute les entrées invalides (`lopdf-0.45.0/src/document.rs:747-770`). | src |
| F13 | `lopdf` 0.45 : `text_string`/`decode_text_string` ré-exportés (`lib.rs:43`), `Stream::new` fixe `/Length` (`object.rs:780`), `From<&str>` produit un Name (`object.rs:106-110`), `Document::is_encrypted` (`document.rs:268`), `remove_object` (`creator.rs:36`). | src |
| F14 | `ctx.open_url` → `egui-winit-0.36.2/src/lib.rs:1130,1394-1400` → `webbrowser::open` (`webbrowser-1.2.4/src/lib.rs:278-336`) ; sous Windows la commande du navigateur par défaut reçoit l'adresse telle quelle (`windows.rs:28-92`) ; le filtre `hardened` (http/https seulement, `lib.rs:318-325`) n'est pas activé par la feature `links` d'`eframe`. | src |
| F15 | Largeurs Helvetica : somme des 95 glyphes ASCII 600,7 pt à 12 pt (table) contre 600,19 pt (boîte englobante `FPDFPageObj_GetBounds` d'un objet texte détaché, qui fonctionne : `ok = 1`). | exp |

Non vérifié (à ne pas affirmer) : rendu dans Acrobat/Foxit/Chrome des annotations écrites par SuniPDF et des réponses `IRT` ; comportement de `lopdf` sur des PDF de plusieurs centaines de Mo ; validation `qpdf --check` ; glyphes Fluent choisis (existence confirmée dans la police, signification visuelle à contrôler).
