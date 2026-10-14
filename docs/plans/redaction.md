# Plan — Caviardage irréversible (« Redact a PDF »)

- Tranche : `redaction` · rédigé le 2026-10-14 · responsable : security-signatures · état : **plan seulement, rien n'est implémenté**.
- Aucune commande `cargo` n'a été exécutée pour ce plan : tout vient de la lecture des sources. Les comportements
  internes de PDFium (C++ absent du poste) sont marqués **[H]** (hypothèse) : le plan ne s'appuie jamais sur eux
  pour garantir l'absence des données, et un test d'absence les couvre.

Abréviations des chemins cités (`fichier:ligne`) :

| Préfixe | Racine |
|---|---|
| `[core]` | `C:/dev_suniworks/SuniPDF/crates/sunipdf-core/src` |
| `[desk]` | `C:/dev_suniworks/SuniPDF/apps/sunipdf-desktop/src` |
| `[cli]` | `C:/dev_suniworks/SuniPDF/apps/sunipdf-cli/src` |
| `[pdfium]` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src` |
| `[hdr]` | `C:/dev_suniworks/SuniPDF/third_party/pdfium/win-x64/include` (en-têtes PDFium 151.0.7881.0) |
| `[lopdf]` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lopdf-0.45.0/src` |
| `[image]` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/image-0.25.10/src` |
| `[egui]` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.2/src` |

---

## 1. Objectif et comportement visible

Équivalent de l'outil PDFCraft **« Redact a PDF »** (catégorie Sécurité).

1. L'utilisateur choisit **Sécurité › Caviarder des zones** (touche `B`, palette Ctrl+K, panneau d'outils,
   palette d'outils flottante). Le curseur devient une croix ; une barre flottante « Caviardage » apparaît à
   droite de la palette d'outils.
2. Il **trace des rectangles** sur la page (glisser). Chaque zone s'affiche en noir semi-transparent avec un
   liseré rouge ; au survol elle devient noire opaque (aperçu du résultat). Un clic sur une zone la retire.
3. Ou il choisit **Caviarder un texte…** : une boîte demande le texte ; toutes les occurrences du document
   (toutes les pages) deviennent des zones, que l'on peut retirer une à une avant d'appliquer.
4. **Rien n'est supprimé tant que le caviardage n'est pas appliqué** : les zones sont un état d'interface.
5. **Appliquer…** (Ctrl+Maj+B) ouvre une confirmation (nombre de zones et de pages, deux options). Après
   validation, le moteur supprime réellement ce qui se trouve sous les zones, réécrit le document, puis
   vérifie lui-même qu'aucun texte n'est plus extractible sous les zones. La barre d'état affiche le bilan.
6. Annulable par Ctrl+Z tant que le document reste ouvert (instantané en mémoire). Le fichier n'est touché
   qu'à l'enregistrement (écriture atomique existante). Enregistrer avec des zones **non appliquées**
   déclenche un avertissement.

Après application : des rectangles noirs, le reste de la page visuellement identique. Dans une ligne de texte
touchée, la partie non caviardée reste lisible mais devient une image (non sélectionnable) : limite assumée (§2.5).

---

## 2. Faisabilité

**Verdict : oui**, hors ligne, sans nouvelle dépendance, avec des licences déjà présentes (pdfium-render
MIT/Apache-2.0, PDFium BSD-3-Clause, lopdf MIT, image MIT/Apache-2.0).

### 2.1 Faits établis par lecture des sources

| # | Fait | Source |
|---|---|---|
| F1 | PDFium n'a pas de fonction « appliquer un caviardage ». pdfium-render sait seulement *lire* une annotation `/Redact` ; aucun créateur correspondant | type `Redacted` : `[pdfium]/pdf/document/page/annotation.rs:126` ; créateurs disponibles : `[pdfium]/pdf/document/page/annotations.rs:200-551` (aucun `create_redact…`) |
| F2 | `PdfDocument::save_to_bytes` appelle `FPDF_SaveAsCopy` avec `flags = 0` (ni `FPDF_INCREMENTAL` ni `FPDF_NO_INCREMENTAL`) | `[pdfium]/pdf/document.rs:334`, `:350`, `:388` ; drapeaux : `[hdr]/fpdf_save.h:46-51` |
| F3 | Retirer un objet de page transfère sa propriété à l'appelant (détruit au `drop`) ; la page doit être régénérée (`FPDFPage_GenerateContent`) sinon la modification est perdue ; après retrait d'un objet texte, les `FPDF_TEXTPAGE` de la page ne sont plus valides | `[pdfium]/pdf/document/page/object/private.rs:149-181`, `:514` ; `[hdr]/fpdf_edit.h:214-227`, `:252-260` |
| F4 | **[H]** PDFium réécrit les flux de contenu *modifiés*, mais ne ramasse pas les objets devenus orphelins (flux de contenu vidé, XObject image retiré, annotation supprimée) et ne nettoie pas forcément `/Resources`. Invérifiable ici : **le plan n'en dépend pas** | — |
| F5 | `lopdf::Document::save_to` écrit seulement les objets de `doc.objects`, un seul `xref`, un seul `%%EOF` : réécriture complète, aucune révision antérieure | `[lopdf]/writer.rs:22-24`, `:65-104` |
| F6 | `prune_objects` supprime tout objet non atteignable depuis le `trailer` ; `delete_object` retire en plus toutes les références à l'objet (tableaux et valeurs de dictionnaire) | `[lopdf]/processor.rs:66-80`, `:83-110` ; parcours : `[lopdf]/document.rs:224-256` |
| F7 | lopdf décode strictement un flux de contenu (erreur si le flux n'est pas analysable jusqu'au bout) ; le décodage tolérant, lui, tronque en silence | `[lopdf]/parser_aux.rs:20-27`, `[lopdf]/parser/mod.rs:830-842` |
| F8 | Les limites d'un objet de page viennent de PDFium (`FPDFPageObj_GetRotatedBounds` pour texte et image, `FPDFPageObj_GetBounds` sinon), en coordonnées de page | `[pdfium]/pdf/document/page/object/private.rs:321-373` ; `[hdr]/fpdf_edit.h:977`, `:999` |
| F9 | Le motif « octets PDFium → lopdf → octets → rechargement PDFium » existe déjà (chiffrement, métadonnées, fusion de flux) | `[core]/security.rs:78-101`, `:232-248` ; `[core]/edit.rs:402-421` |
| F10 | La boîte de page (intersection MediaBox ∩ CropBox, espace utilisateur) et la rotation sont lisibles ; la largeur/hauteur suivent la rotation | `[pdfium]/pdf/document/page/boundaries.rs:248-262`, `[hdr]/fpdfview.h:765-775` ; `[pdfium]/pdf/document/page.rs:292`, `:300` ; `[hdr]/fpdf_edit.h:174-182` |

Conséquence de F4 : **supprimer un objet avec l'outil d'édition existant (`Document::delete_object`,
`[core]/edit.rs:454-460`) n'est pas un caviardage.** À écrire dans `KNOWN_ISSUES.md` (§11).

### 2.2 Méthode retenue (une seule voie de code, deux modes)

Pour chaque page portant des zones :

- **A. PDFium** — tout objet de page dont les limites croisent une zone (*victime*) est **retiré en entier**
  (texte visible ou invisible, image, tracé, dégradé, XObject formulaire, objet non reconnu). Avant le retrait,
  la page est rendue en image et les zones y sont peintes en noir ; pour chaque zone, une **vignette** (image
  découpée dans ce rendu) couvrant la zone et l'empreinte de ses victimes est ajoutée au premier plan. La
  vignette restitue l'aspect de ce qui dépassait de la zone ; les pixels de la zone sont noirs **dans les
  données mêmes de l'image**.
- **B. lopdf** (passe d'assainissement, obligatoire) — annotations et champs de formulaire sous les zones
  supprimés ; dictionnaire de page réduit à une liste blanche de clés (plus de `/Thumb`, `/PieceInfo`…) ;
  ressources de la page filtrées aux seuls noms encore utilisés ; ressources retirées que plus aucune page ne
  dessine supprimées ; ramasse-miettes ; réécriture complète.
- **C. Vérification** — le résultat est rechargé dans PDFium ; s'il reste du texte extractible sous une zone,
  erreur typée et document inchangé.

Mode **« page en image »** (option) : même code avec « victimes = tous les objets » et « empreinte = la page » ;
toutes les annotations de la page sont supprimées (leur aspect est incrusté dans l'image). C'est le repli
quand le mode précis refuse un document, et le choix « sécurité maximale ».

### 2.3 Options écartées, et pourquoi

| Option | Raison du rejet |
|---|---|
| Découper un objet texte caractère par caractère | Aucune API pour recréer un fragment avec la police incorporée d'origine et le positionnement par glyphe ; `set_text` réencode via Unicode et échoue sur les sous-ensembles de police (limite connue A6 de `KNOWN_ISSUES.md`). Un découpage raté laisserait du texte en silence |
| Réencoder l'image d'origine (`get_raw_image` / `set_image`) | `get_raw_image` ignore filtres, masque et transformation (`[pdfium]/pdf/document/page/object/image.rs:271-290`) : il faudrait inverser la matrice ; `set_image` réécrit en BGRA (`:686-720`) et perd masques et espaces colorimétriques ; l'image peut être partagée par d'autres pages (le noir y apparaîtrait) ; l'ancien flux peut rester orphelin **[H]**. La vignette rendue est fidèle à ce qui est affiché et ne touche aucun objet partagé |
| Marques persistantes en annotations `/Redact` | Pas de créateur dans pdfium-render (F1) |
| Rectangle noir superposé | Ce n'est pas un caviardage |
| Crate séparé `sunipdf-security` | Le code doit accéder à `Document::inner` (privé à `sunipdf-core`) : nouveau **module** `[core]/redact.rs`. À valider par l'architecte (`ARCHITECTURE.md` prévoyait le crate en P5) |

### 2.4 Garanties visées et preuves

| Vecteur de fuite | Mécanisme | Test (§8) |
|---|---|---|
| Texte sous la zone (visible, invisible, couche OCR) | retrait de l'objet texte entier + vérification C | T1, T2 |
| Image sous la zone | retrait de l'objet, pixels noirs dans la vignette, XObject supprimé (B3, B4) | T2, U3 |
| Tracé vectoriel | retrait de l'objet, vignette | T2 |
| Flux de contenu vidé resté orphelin [H] | B6 (`prune_objects`) | T4 |
| Annotation sous la zone, son popup, ses réponses | B1 | T1, U4 |
| Valeur d'un champ de formulaire sous la zone | B1 (champ terminal retiré de `/AcroForm`) | T3, U4 |
| `/Thumb`, `/PieceInfo` de la page | B2 (liste blanche) | T1 |
| Objets non référencés, révisions antérieures | F5 + F6 | T1, T6 |
| Métadonnées, arbre de structure (`/Alt`, `/ActualText`) | B5, option cochée par défaut | T1 |
| Zone mal placée sur une page pivotée | conversion affichage ↔ page | T5, U1 |
| Détecteur de test aveugle au texte hexadécimal écrit par PDFium | contrôle positif | T1 (étape 2) |

### 2.5 Limites honnêtes (à publier telles quelles)

1. **Granularité** : un objet texte touché disparaît en entier de la couche texte ; son reste est une image (ni
   sélectionnable ni trouvable). Idem pour une image ou un tracé partiellement couvert (vignette à 200 ppp).
2. **Texte d'une image numérisée sans OCR** : invisible pour « Caviarder un texte… » ; il faut tracer la zone.
   Une couche OCR décalée par rapport à l'image peut échapper à une zone tracée : le mode « page en image »
   la supprime dans tous les cas.
3. **Polices incorporées** : si la police du texte caviardé sert encore ailleurs, son sous-ensemble de glyphes
   reste dans le fichier (il révèle des caractères utilisés, ni leur ordre ni leur position).
4. **Non modifiés** : signets (un titre peut reprendre le texte caviardé), pièces jointes, JavaScript,
   destinations nommées, contenu des pages non marquées, annotations `/Redact` posées par un autre logiciel
   (traitées comme des annotations ordinaires).
5. **Balisage d'accessibilité** : supprimé pour tout le document quand l'option « informations cachées » est
   cochée (défaut) ; conservé sinon, avec le risque qu'un texte alternatif décrive le contenu caviardé.
6. **Document chiffré** : le résultat n'est plus chiffré (lopdf déchiffre, `[core]/security.rs:78-94`) ;
   l'interface le signale, il faut réappliquer « Protéger ».
7. **Signatures numériques** : toute signature existante devient invalide (réécriture complète).
8. **Mémoire et disque** : l'historique d'annulation et les onglets suspendus gardent l'état d'avant en
   mémoire vive (jamais écrit sur disque par SuniPDF) ; l'ancien fichier remplacé par l'enregistrement
   atomique n'est pas effacé du support de façon sécurisée.
9. **Hypothèse restante** : les limites d'objet de PDFium englobent l'encre de l'objet. Pour le texte elle
   est contrôlée à l'exécution (étape C) ; pour images et tracés les limites sont géométriques.
10. **Documents refusés** : si le flux de contenu d'une page ne se décode pas strictement (F7), le mode précis
    renvoie `redaction_incomplete` sans rien modifier ; l'utilisateur coche « page en image ».

---

## 3. API moteur (`sunipdf-core`)

Nouveau fichier **`crates/sunipdf-core/src/redact.rs`** ; dans `[core]/lib.rs` : `pub mod redact;` (après
`pub mod pages;`, `:16`) et `pub use redact::{RedactMark, RedactOptions, RedactReport};` (avec les `pub use`
de `:21-30`). Aucune chaîne d'interface.

### 3.1 Types et signatures

```rust
/// Zone à caviarder. `rect` : coordonnées de page (espace utilisateur PDF, points), comme `TextMatch::rects`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactMark { pub page: usize, pub rect: PointRect }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedactOptions {
    /// Remplacer tout le contenu de chaque page marquée par une image (aucun texte ne subsiste sur ces pages).
    pub rasterize_pages: bool,
    /// Supprimer aussi /Info, XMP, l'arbre de structure et /PieceInfo du catalogue.
    pub clean_hidden_data: bool,
    /// Résolution des vignettes et des pages converties (bornée à 72..=600).
    pub raster_dpi: f32,
}
impl Default for RedactOptions { /* false, true, 200.0 */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RedactReport {
    pub marks: usize, pub pages: usize, pub objects_removed: usize, pub annotations_removed: usize,
    pub fields_removed: usize, pub pages_rasterized: usize,
    pub hidden_data_cleaned: bool, pub encryption_removed: bool,
}

impl Document<'_> {
    /// Caviarde ; en cas d'erreur l'appelant doit restaurer son instantané (le worker le fait, §4).
    pub fn redact(&mut self, marks: &[RedactMark], options: &RedactOptions) -> Result<RedactReport, EngineError>;
    /// Une zone par segment de ligne de chaque occurrence, agrandie de 1 pt (coordonnées de page).
    pub fn redact_marks_for_text(&self, query: &str, match_case: bool) -> Result<Vec<RedactMark>, EngineError>;
    /// Rectangle tel que vu à l'écran (points, origine en bas à gauche de la page affichée) → coordonnées de page.
    pub fn display_rect_to_page(&self, page_index: usize, rect: PointRect) -> Result<PointRect, EngineError>;
    pub fn page_rect_to_display(&self, page_index: usize, rect: PointRect) -> Result<PointRect, EngineError>;
}
```

Constantes privées : `TEXT_PAD_PT = 1.0`, `RASTER_MAX_PIXELS = 40_000_000.0` (≈ 160 Mo RGBA ; commentaire
`ponytail:` — plafond fixe, à rendre réglable si des plans A0 l'exigent), `JPEG_QUALITY = 90`.

### 3.2 Nouvelle variante d'erreur (`[core]/error.rs`)

Dans l'enum (`:6-25`) : `#[error("redaction could not be verified; the document was left unchanged")] RedactionIncomplete,`
et dans `code()` (`:29-41`) : `Self::RedactionIncomplete => "redaction_incomplete"`. Elle couvre les deux cas
où la garantie ne peut pas être donnée : flux de contenu non décodable strictement (B3/B4), texte résiduel (C).

### 3.3 Espaces de coordonnées (pages pivotées, boîte décalée)

L'interface travaille en **espace d'affichage** : `to_pdf` (`[desk]/annotate.rs:99-104`) donne des points avec
l'origine en bas à gauche de la page *telle qu'affichée* et utilise les tailles de `DocumentInfo::page_sizes`,
qui suivent la rotation. Les objets de page, le texte et `/Rect` des annotations sont en **espace de page**. Les
outils actuels confondent les deux (sans effet sur une page non pivotée à origine (0,0)) ; pour un caviardage,
un décalage silencieux est inacceptable.

Fonction pure privée, avec `b` = boîte de page en espace de page (`page.boundaries().bounding()?.bounds`,
`[pdfium]/pdf/document/page/boundaries.rs:248` ; accesseurs `left()`… `[pdfium]/pdf/rect.rs:119`) et la rotation
lue par `Document::page_rotation` (`[core]/pages.rs:151`) :

| Rotation `/Rotate` | affichage → page | page → affichage |
|---|---|---|
| 0 | `x = b.left + xd`, `y = b.bottom + yd` | `xd = x − b.left`, `yd = y − b.bottom` |
| 90 (horaire) | `x = b.right − yd`, `y = b.bottom + xd` | `xd = y − b.bottom`, `yd = b.right − x` |
| 180 | `x = b.right − xd`, `y = b.top − yd` | `xd = b.right − x`, `yd = b.top − y` |
| 270 | `x = b.left + yd`, `y = b.top − xd` | `xd = b.top − y`, `yd = x − b.left` |

Un rectangle est converti par deux coins opposés puis renormalisé (min/max). **Cette table est déduite de la
définition de `/Rotate` (sens horaire, `[hdr]/fpdf_edit.h:174-182`), pas observée** : le test T5 la confronte
au rendu réel de PDFium ; s'il contredit la table, c'est la table qu'on corrige.

### 3.4 `redact_marks_for_text`

`self.search(query, match_case)?` (`[core]/text.rs:67`) ; pour chaque `TextMatch` et chaque rectangle : un
`RedactMark { page, rect agrandi de TEXT_PAD_PT de chaque côté }`. Requête vide → liste vide (comportement de
`search_page`, `[core]/text.rs:37-39`).

### 3.5 `redact` : déroulé exact

**Validation (avant toute modification)** : `marks` vide → `EngineError::Other("nothing to redact")` ;
`page >= page_count()` → `PageOutOfRange(page)` ; rectangle normalisé, coordonnées finies, largeur et hauteur
≥ 0,1 pt sinon `Other("invalid redaction rectangle")`. Zones groupées par page dans un
`BTreeMap<usize, Vec<PointRect>>`.

#### Phase A — par page, dans PDFium (`fn redact_page_objects`)

```rust
let mut page = self.page_handle(page_index)?;                       // [core]/pages.rs:282
let rotation = page.rotation()?;                                    // [pdfium]/pdf/document/page.rs:292
page.set_rotation(PdfPageRenderRotation::None);                     // :300 — affichage == espace de page
let result = (|| -> Result<usize, EngineError> {
    let bbox = from_pdf_rect(page.boundaries().bounding()?.bounds); // boundaries.rs:248
    let (w, h) = (bbox.right - bbox.left, bbox.top - bbox.bottom);

    // 1. Victimes : (index, limites, est_une_image). Limites illisibles → la page entière (prudence).
    let mut victims = Vec::new();
    for (i, o) in page.objects().iter().enumerate() {               // objects/common.rs:89
        let b = o.bounds().map(quad_to_rect).unwrap_or(bbox);       // object.rs:679 ; private.rs:321
        if rasterize || rects.iter().any(|r| intersects(r, &b)) {
            victims.push((i, b, o.object_type() == PdfPageObjectType::Image));   // object.rs:399
        }
    }
    // 2. Empreintes : une par zone = zone ∪ limites de ses victimes, bornée à la page ; mode image : la page.
    // 3. Rendu AVANT retrait.
    let scale = (dpi / 72.0).min((RASTER_MAX_PIXELS / (w * h)).sqrt())
                            .min(crate::MAX_RENDER_SIDE_PX as f32 / w.max(h));
    let (pw, ph) = ((w * scale).round() as i32, (h * scale).round() as i32);
    let config = PdfRenderConfig::new().set_target_size(pw, ph)     // render_config.rs:76, :187
        .render_form_data(rasterize).render_annotations(rasterize); // :393, :402
    let mut img = page.render_with_config(&config)?.as_image()?.into_rgba8();
                                              // page.rs:569 ; bitmap.rs:329 ; [image]/images/dynimage.rs:418
    // 4. Noir dans les pixels : pour chaque zone, rectangle de pixels arrondi VERS L'EXTÉRIEUR.
    //    x0 = floor((r.left − bbox.left)·sx), x1 = ceil((r.right − bbox.left)·sx),
    //    y0 = floor((bbox.top − r.top)·sy),   y1 = ceil((bbox.top − r.bottom)·sy), bornés à l'image ;
    //    sx = pw / w, sy = ph / h. Chaque pixel reçoit [0, 0, 0, 255] ([image]/images/buffer.rs:986).
    // 5. Retrait groupé, régénération différée.
    page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);  // page.rs:909
    for (i, ..) in victims.iter().rev() {
        page.objects_mut().remove_object_at_index(*i)?;             // objects/common.rs:368, :664
    }
    // 6. Une vignette par empreinte (mêmes arrondis vers l'extérieur) :
    //    sous-image = image::imageops::crop_imm(&img, x0, y0, x1−x0, y1−y0).to_image()
    //                 ([image]/imageops/mod.rs:56, [image]/images/sub_image.rs:78)
    //    rectangle de page exactement couvert : left = bbox.left + x0/sx, right = bbox.left + x1/sx,
    //                                           top = bbox.top − y0/sy, bottom = bbox.top − y1/sy
    //    objet image puis object.translate(left, bottom) et page.objects_mut().add_image_object(object)
    //    (même séquence que `add_image`, [core]/edit.rs:478-485 ; objects/common.rs:315).
    page.regenerate_content()?;                                      // page.rs:942
    Ok(victims.len())
})();
page.set_rotation(rotation);                                         // toujours restaurée, même en erreur
let removed = result?;
```

Précisions :

- **Régénération différée** : par défaut pdfium-render régénère le contenu à chaque retrait
  (`[pdfium]/pdf/document/page/object/private.rs:156-168`) ; avec la stratégie `Manual` un seul
  `FPDFPage_GenerateContent` suffit (indispensable en mode image : des milliers d'objets). La stratégie est
  attachée à ce `PdfPage` (`[pdfium]/pdf/document/page.rs:909-924`) et le `drop` ne régénère rien en `Manual`
  (`:997-1005`) : l'appel explicite à `regenerate_content` est donc obligatoire.
- **Rendu sans annotations ni champs en mode précis** : avec `render_form_data(false)` pdfium-render passe par
  `FPDF_RenderPageBitmapWithMatrix` (`[pdfium]/pdf/document/page.rs:702-713`,
  `[pdfium]/pdf/document/page/render_config.rs:874-895`). Les annotations conservées ne doivent pas être
  incrustées dans les vignettes (double tracé). En mode image, au contraire, on rend **avec** annotations et
  champs (configuration de `render_page`, `[core]/lib.rs:237-240`) puisqu'ils sont tous supprimés ensuite.
- **Encodage de la vignette** : si une victime *image* couvre au moins la moitié de l'empreinte (photo, page
  numérisée) → JPEG : `image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY)` puis
  `.encode_image(&rgb)` (`[image]/codecs/jpeg/encoder.rs:398`, `:533` ; conversion `into_rgb8`,
  `[image]/images/dynimage.rs:382`), objet créé par
  `PdfPageImageObject::new_from_jpeg_reader(&self.inner, std::io::Cursor::new(buf))`
  (`[pdfium]/pdf/document/page/object/image.rs:167-193` ; données copiées tout de suite, `[hdr]/fpdf_edit.h:744-761`)
  puis `object.scale(largeur, hauteur)`. Sinon sans perte :
  `PdfPageImageObject::new_with_size(&self.inner, &DynamicImage::ImageRgba8(sub), w, h)` (`image.rs:259-271`).
  Raison : une page numérisée ré-encodée sans perte à 200 ppp pèse plusieurs Mo.
- **Ordre de dessin** : les vignettes sont ajoutées en dernier, donc au premier plan. Découpées dans un rendu
  complet de la page, elles contiennent déjà l'aspect des objets conservés qu'elles recouvrent.
- **XObject formulaire** : vu comme un seul objet opaque ; s'il croise une zone il est retiré en entier et
  remplacé par sa vignette (au pire la page entière : dégradation naturelle vers le mode image).
- `quad_to_rect` et `from_pdf_rect` existent en privé (`[core]/edit.rs:98-105`, `[core]/annot.rs:39-46`) : les
  passer en `pub(crate)` plutôt que les dupliquer.
- `intersects(a, b)` = recouvrement strict d'aire non nulle (`a.left < b.right && a.right > b.left && …`).

#### Phase B — assainissement lopdf (`fn sanitize`, pur, testable sans PDFium)

```rust
pub(crate) struct Sanitized { pub bytes: Vec<u8>, pub annotations_removed: usize,
                              pub fields_removed: usize, pub encryption_removed: bool }
pub(crate) fn sanitize(input: &[u8], password: Option<&str>,
                       pages: &BTreeMap<usize, Vec<PointRect>>, clean_hidden: bool)
    -> Result<Sanitized, EngineError>;
```

Entrée : `self.snapshot()?` (`[core]/pages.rs:246`) après la phase A. Pour une page en mode image, `redact`
passe un rectangle unique « infini » (`f32::MIN..f32::MAX`) : toutes ses annotations sont alors retirées.

- **B0. Chargement** : `crate::security::load(input, password)?` (`[core]/security.rs:78-94`, déchiffre si
  besoin) ; `encryption_removed = doc.was_encrypted()` (`[lopdf]/document.rs:273`) ;
  `doc.get_pages()` (`:533`) donne `numéro (base 1) → ObjectId`.
- **B1. Annotations et champs** (par page marquée) :
  1. copier le tableau `/Annots` : référence vers un tableau ou tableau direct, entrées par référence ou
     dictionnaires directs (les deux formes sont légales, cf. `[lopdf]/document.rs:747-768`) ;
  2. une entrée est *retirée* si son `/Rect` (4 nombres lus par `Object::as_float`, `[lopdf]/object.rs:177`,
     après `doc.dereference`, `[lopdf]/document.rs:147`) croise une zone, **ou s'il est absent ou illisible** ;
  3. point fixe : est aussi retirée toute entrée dont `/Parent` (popup) ou `/IRT` (réponse) désigne une entrée
     retirée, et le `/Popup` d'une entrée retirée ;
  4. pour chaque entrée retirée de sous-type `/Widget` : remonter `/Parent` jusqu'au premier dictionnaire
     portant `/T` (champ terminal), puis tant que le parent porte `/V` ou `/DV` (valeur héritée) ; noter cet
     identifiant comme *champ à supprimer* **et collecter tout de suite son sous-arbre par `/Kids`** (avant
     l'étape 6, qui peut supprimer le widget lui-même quand il est fusionné avec son champ) ;
  5. réécrire `/Annots` de la page en tableau direct des entrées conservées (clé retirée si vide) ;
  6. pour chaque identifiant retiré : `doc.delete_object(id)` (`[lopdf]/processor.rs:83-110`), qui ôte aussi
     les références restantes (`/IRT`, `/Popup`, arbre de structure).
  Après toutes les pages : pour chaque identifiant des sous-arbres collectés encore présent,
  `doc.delete_object(id)` (retire les références de `/AcroForm /Fields`, `/CO`, `/Kids` et
  des `/Annots` des autres pages) ; s'il y en a eu, retirer `/XFA` du dictionnaire `/AcroForm` (copie XML des
  valeurs). `fields_removed` = nombre de champs terminaux supprimés. Sur-suppression assumée : un champ
  affiché à plusieurs endroits disparaît partout.
- **B2. Liste blanche du dictionnaire de page** : ne garder que `Type, Parent, MediaBox, CropBox, BleedBox,
  TrimBox, ArtBox, Rotate, UserUnit, Contents, Resources, Annots, Group, Tabs` (`Dictionary::iter` et `remove`,
  `[lopdf]/object.rs:400`, `:386`). Tombent : `Thumb`, `PieceInfo`, `Metadata`, `B`, `AA`, `AF`, `StructParents`…
- **B3. Filtrage des ressources de la page** :
  1. *ressources effectives* : `/Resources` de la page, sinon du plus proche ancêtre par `/Parent`
     (l'héritage PDF remplace, il ne fusionne pas), déréférencé et **cloné** ;
  2. *noms utilisés* : `doc.get_page_content(page_id)` (`[lopdf]/document.rs:598-611`) puis
     `lopdf::content::Content::decode_strict(&data)` (`[lopdf]/parser_aux.rs:25`) ; échec →
     `Err(EngineError::RedactionIncomplete)`. Parcours des `Operation { operator, operands }`
     (`[lopdf]/content.rs:7-10`) :

     | Opérateur | Opérande | Catégorie de `/Resources` |
     |---|---|---|
     | `Do` | 0 | `XObject` |
     | `Tf` | 0 | `Font` |
     | `gs` | 0 | `ExtGState` |
     | `sh` | 0 | `Shading` |
     | `cs`, `CS` | 0 | `ColorSpace` |
     | `scn`, `SCN` | dernier, s'il est un nom | `Pattern` |
     | `BDC`, `DP` | 1, s'il est un nom | `Properties` |
     | `BI` (image en ligne) | `/CS` ou `/ColorSpace` du dictionnaire, s'il est un nom | `ColorSpace` |

     Pour chaque XObject utilisé qui est un formulaire **sans** `/Resources` propre (il hérite de celles de la
     page) : décoder aussi son flux (`Stream::decompressed_content`, `[lopdf]/object.rs:885`) et ajouter ses
     noms (récursif, avec ensemble des identifiants déjà vus) ;
  3. pour chacune des sept catégories : déréférencer le sous-dictionnaire, le cloner, n'y garder que les noms
     utilisés ; chaque entrée retirée dont la valeur est une référence est ajoutée aux *candidats*
     `(catégorie, ObjectId)` ;
  4. écrire le résultat comme `/Resources` **direct** de la page (`Dictionary::set`, `[lopdf]/object.rs:370`).
- **B4. Ressources retirées que plus personne ne dessine** — indispensable quand un dictionnaire de
  ressources est partagé par toutes les pages (cas courant) : sans cela l'image retirée resterait atteignable.
  Pour chaque candidat `(cat, id)` : il est *encore dessiné* si une page (marquée ou non) le voit sous un nom
  `n` de ses ressources effectives **et** utilise `n` dans la catégorie `cat` (calcul de B3.2, mis en cache par
  page ; seules les pages dont les ressources référencent `id` sont décodées ; échec de décodage →
  `Err(RedactionIncomplete)`). S'il n'est plus dessiné : `doc.delete_object(id)`.
- **B5. Informations cachées** (si `clean_hidden`) : corps de `strip_metadata` (`[core]/security.rs:232-248`)
  extrait en `pub(crate) fn strip_metadata_in(doc: &mut lopdf::Document)` et réutilisé ; puis
  `catalog_mut()` (`[lopdf]/document.rs:525`) : retirer `StructTreeRoot`, `MarkInfo`, `PieceInfo`.
- **B6.** `doc.prune_objects()` (`[lopdf]/processor.rs:66-80`) puis `crate::security::save(doc)`
  (`[core]/security.rs:96-101`).

#### Phase C — rechargement et vérification

```rust
let previous_password = self.password.clone();
let outcome = (|| -> Result<(), EngineError> {
    if sanitized.encryption_removed { self.restore_with_password(sanitized.bytes, None)?; } // pages.rs:256
    else { self.restore(sanitized.bytes)?; }                                                 // pages.rs:266
    let residual = marks.iter().any(|m| self.text_in_rect(m.page, m.rect).map_or(true, |s| !s.text.is_empty()))
        || rasterized_pages.iter().any(|p| self.page_text(*p).map_or(true, |t| !t.trim().is_empty()));
    if residual { Err(EngineError::RedactionIncomplete) } else { Ok(()) }
})();
if outcome.is_err() {
    // Sinon le worker ne pourrait pas recharger son instantané (chiffré) avec l'ancien mot de passe.
    self.password = previous_password;
}
outcome?;
```

Une erreur de lecture pendant la vérification compte comme un échec (`map_or(true, …)`) : on échoue fermé.

`text_in_rect` (`[core]/annot.rs:70-113`) parcourt tous les caractères de la page et retient ceux dont le
centre est dans le rectangle ; `page_text` : `[core]/text.rs:24`. La vérification se fait **après** le
rechargement, donc sur l'état exact qui sera enregistré (et F3 : les pages de texte antérieures au retrait
sont invalides).

### 3.6 Instantanés, annulation, enregistrement

- `redact` remplace `self.inner` par un document rechargé, comme `BytesOp` (`[desk]/worker.rs:526-555`) :
  aucun `PdfPage` ne doit survivre à l'appel (les `page_handle` sont locaux à la phase A).
- Annulation : instantané pris par le worker avant l'appel (§4) ; « Annuler » restaure le contenu d'avant —
  attendu et documenté (§2.5 point 8).
- Enregistrement : `Document::save_as` (`[core]/pages.rs:276-280`) écrit ce que PDFium a chargé depuis les
  octets assainis ; PDFium ne peut rien ressusciter puisqu'il n'a jamais reçu l'ancien contenu. Les tests
  cherchent néanmoins dans le fichier produit par `save_as`, pas seulement dans les octets lopdf.
- Journalisation : compter (zones, objets), ne **jamais** journaliser le texte recherché ni un contenu.

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

Le caviardage n'entre pas dans `EditOp` : `apply` renvoie `Result<(), _>` (`:415`) et on a besoin du bilan.
Deux requêtes et deux réponses, plus une petite généralisation de `State::edit`.

```rust
pub enum Request {
    // …
    /// Occurrences d'un texte, converties en zones d'affichage. `verify` : contrôle après application.
    RedactSearch { query: String, match_case: bool, verify: bool },
    /// Zones en espace d'affichage (page, rectangle).
    Redact { marks: Vec<(usize, PointRect)>, options: RedactOptions },
}
pub enum Response {
    // …
    RedactMatches { query: String, verify: bool, marks: Vec<(usize, PointRect)> },
    Redacted { state: DocState, report: RedactReport },
}
```

`State::edit` (`:372-393`) devient un appel à une version générique (même logique : instantané, action,
`push_undo` + `invalidate` en cas de succès, `restore` en cas d'échec) :

```rust
fn edit_with<T>(&mut self, f: impl FnOnce(&mut Document<'_>) -> Result<T, EngineError>)
    -> Result<(T, DocState), EngineError> { /* corps actuel de `edit`, `apply(doc, op)` remplacé par `f(doc)` */ }

fn edit(&mut self, op: EditOp) -> Response {
    match self.edit_with(|doc| apply(doc, op)) {
        Ok(((), state)) => Response::DocumentChanged(state),
        Err(e) => Response::OperationFailed(e),
    }
}
```

Dans la boucle `run` (à côté de `Request::Edit`, `:717`) :

```rust
Request::Redact { marks, options } => match state.edit_with(|doc| {
    let marks = marks.iter()
        .map(|(page, rect)| Ok(RedactMark { page: *page, rect: doc.display_rect_to_page(*page, *rect)? }))
        .collect::<Result<Vec<_>, EngineError>>()?;
    doc.redact(&marks, &options)
}) {
    Ok((report, state)) => Response::Redacted { state, report },
    Err(e) => Response::OperationFailed(e),
},
Request::RedactSearch { query, match_case, verify } => {
    let Some(doc) = &state.document else { continue };
    let marks = doc.redact_marks_for_text(&query, match_case).unwrap_or_default().into_iter()
        .filter_map(|m| doc.page_rect_to_display(m.page, m.rect).ok().map(|r| (m.page, r)))
        .collect();
    Response::RedactMatches { query, verify, marks }
}
```

Aucun `BatchOp` : le caviardage modifie le document ouvert (annulable), il ne produit pas de fichier.
Tout le travail lourd reste sur le thread moteur ; l'interface affiche l'indicateur d'activité (§5.6).

---

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Commandes (`[core]/command.rs`)

Variantes ajoutées à l'enum (`:5-69`, avant `About`) et entrées `cmd!` après `StripMetadata` (`:422-429`) :

| Variante | id | Clé du libellé | Raccourci | `needs_document` | Menu |
|---|---|---|---|---|---|
| `ToolRedact` | `tool.redact` | `menu.security.redact` | `shortcut.tool_redact` (`B`) | oui | `Security` |
| `RedactSearch` | `security.redact_search` | `menu.security.redact_search` | — | oui | `Security` |
| `RedactApply` | `security.redact_apply` | `menu.security.redact_apply` | `shortcut.redact_apply` (Ctrl+Maj+B) | oui | `Security` |

Le test `every_command_variant_is_registered` (`:643-649`) compte le menu `File` : inchangé. Le menu
« Sécurité » est construit depuis le registre (`[desk]/shell.rs:470`) : les trois commandes y apparaissent
sans autre code.

### 5.2 Panneau d'outils, icône, couleur (`[desk]/shell.rs`, `[desk]/icons.rs`)

- `ToolTab::Security` (`shell.rs:103-108`) devient
  `[Protect, Unprotect, ToolRedact, RedactSearch, RedactApply, StripMetadata, Properties]` ; `ToolTab::All`
  (`:48-76`) reçoit `ToolRedact` après `Protect`.
- `command_icon` (`:246-309`, `match` exhaustif : oubli = erreur de compilation) :
  `ToolRedact => (glyph::REDACT, "█")`, `RedactSearch => (glyph::FIND, "🔍")`, `RedactApply => (glyph::REDACT, "█")`.
- `icons.rs` (module `glyph`, `:12-70`) : `pub const REDACT: &str = "\u{E73B}";`. Le point de code existe dans
  `SegoeIcons.ttf` **et** `segmdl2.ttf` de ce poste (vérifié par requête `GlyphTypeface` en PowerShell) ; son
  dessin (carré plein, « CheckboxFill ») reste à confirmer sur la capture S1.
- `command_color` (`:150-186`) : rien à faire, le menu `Security` donne déjà `theme::CAT_SECURITY`.
- **Si la tranche `tool-hub` a atterri avant** (`docs/plans/tool-hub.md` §5.9) : au lieu de `ToolTab`, passer
  `Workspace::Redact` à `entry: Some(Entry::Mode(Command::HubRedact))`,
  `commands: &[ToolRedact, RedactSearch, RedactApply]`, icône `(glyph::REDACT, "█")`, et ajouter `HubRedact`
  (id `hub.redact`, libellé `hub.redact.name` déjà prévu par cette tranche, menu `Tools`).
- Carte d'accueil « Sécuriser » : la valeur de `home.card.security.desc` gagne « · Caviardage » (§7).

### 5.3 Outil (`[desk]/annotate.rs`, `[desk]/editor.rs`)

- `Tool::Redact` ajouté à l'enum (`annotate.rs:12-25`), à `Tool::ALL` (`[Tool; 12]`, `:28-40`) et à
  `Tool::command()` (`:42-56`) → `Command::ToolRedact`. Il apparaît alors dans la palette flottante
  (`tool_palette`, `:108-153`) ; étendre la condition du séparateur (`:130`) à `Tool::Redact`.
- `execute` : ajouter `Command::ToolRedact` au bras des outils (`[desk]/app.rs:923-942`).
- `finish_drag` (`annotate.rs:190-224`) : bras explicite `Tool::Redact => self.redact_drag_finished(drag)`
  **avant** le bras `_`, qui sinon enverrait un `TextInRect` (`:214-222`).
- `page_cursor` (`editor.rs:186-213`) : `Tool::Redact` rejoint le bras `CursorIcon::Crosshair` (`:188-193`).
- `Tool::is_edit` (`editor.rs:153-165`) : inchangé (la barre d'objet reste masquée, `:609-612`).

### 5.4 État (nouveau fichier `apps/sunipdf-desktop/src/redact.rs`, `mod redact;` dans `main.rs`)

```rust
pub struct RedactState {
    /// Zones marquées, non appliquées : (page, rectangle en espace d'AFFICHAGE, points, origine bas-gauche).
    pub marks: Vec<(usize, PointRect)>,
    pub search_open: bool,
    pub search_query: String,
    pub search_match_case: bool,
    /// Dernier texte marqué : recherché à nouveau après application (contrôle).
    pub last_query: Option<(String, bool)>,
    pub confirm_open: bool,
    pub rasterize: bool,
    pub clean_hidden: bool,          // vrai par défaut (impl Default manuelle)
    pub applying: bool,
    /// Enregistrement demandé avec des zones non appliquées : `Some(ask_path)`.
    pub save_prompt: Option<bool>,
    pub save_confirmed: bool,
}
```

- Champ `pub redact: crate::redact::RedactState` dans `SuniApp` (`[desk]/app.rs:82-127`, initialisé dans `new`,
  `:146-154`) **et** dans `TabState` (`[desk]/tabs.rs:10-25`), échangé dans `take_tab` / `put_tab` (`:51`,
  `:80`) comme `annotate` et `organize` : les zones appartiennent à un document.
- `close_document` (`app.rs:239-266`) : `self.redact = Default::default();`.

### 5.5 Interaction sur la page (`[desk]/app.rs::document_area`, `:1058-1232`)

- Après `paint_object_selection` (`:1144`) : `self.paint_redact_marks(&painter, page_rect, page_index, size.height_pt);`.
  Pour chaque zone de la page : `painter.rect_filled` noir d'alpha 110, opaque si le pointeur
  (`painter.ctx().pointer_hover_pos()`, `[egui]/painter.rs:128`, `[egui]/context.rs:3033`) est dessus ;
  `painter.rect_stroke` 1,5 px `theme::ERROR` (`[egui]/painter.rs:397`, `:406`). Le glissement en cours
  (`self.annotate.drag`) est dessiné de la même façon. Conversion point → écran identique à `paint_matches`
  (`app.rs:1474-1478`).
- Clic (`DragEvent::Click`, `:1207-1222`) : bras `Tool::Redact => self.redact_click(page_index, pt)` — retire
  la zone la plus récente contenant le point, état `security.redact.status.removed`.
- Fin de glissement : `redact_drag_finished(drag)` — ignore un rectangle de moins de 2 pt (même critère
  `tiny` que `finish_drag`, `annotate.rs:192`), borne à la largeur de la page (`to_pdf` ne borne pas x à droite,
  `annotate.rs:103`), ajoute `(drag.page, drag.rect())`. **Aucune requête au moteur.**
- Après `object_bar` (`:1231`) : `self.redact_bar(ctx, palette_origin + egui::vec2(46.0, 0.0));` — affichée si
  l'outil courant est `Redact` (jamais en même temps que la barre d'objet). Même gabarit que `object_bar`
  (`[desk]/editor.rs:618-627`) : icône, texte `security.redact.bar.count` (ou `…bar.empty`), boutons
  « Rechercher un texte… » (`RedactSearch`), « Tout effacer », « Appliquer… » (`RedactApply`), infobulle
  `security.redact.bar.hint`. Tous réels : pas de bouton factice.
- **Si la tranche `continuous-view` a atterri avant** : mêmes appels, mais par page, dans ses fonctions
  `paint_slot` / `handle_page_event` ; les données ne changent pas.

### 5.6 Commandes, clavier, réponses (`[desk]/app.rs`)

- `execute` (`:793`) :
  - `RedactSearch` → `tool = Tool::Redact`, `search_open = true`, `search_query` prérempli par
    `self.find.query` s'il est vide ;
  - `RedactApply` → si aucune zone : `self.error = Some("error.redact_no_marks")`, sinon `confirm_open = true`.
- `handle_input` :
  - garde des boîtes modales (`:591-598`) : ajouter `|| self.redact.search_open || self.redact.confirm_open
    || self.redact.save_prompt.is_some()` ;
  - `key(Modifiers::COMMAND | Modifiers::SHIFT, Key::B, Command::RedactApply);` **avant tout** `Ctrl+B`, puis
    `key(Modifiers::NONE, Key::B, Command::ToolRedact);`. `consume_key` ignore Maj et Alt en trop mais refuse
    Ctrl quand le motif n'en veut pas (`[egui]/input_state/mod.rs:722`, `[egui]/data/input/modifiers.rs:211-220`,
    `:300-315`). Attention : le plan `bookmarks-links` réserve `K` et `Ctrl+B`, `sign-image` réserve `S`.
- `handle_responses` (`:327`) :
  - extraire le corps du bras `DocumentChanged` (`:422-459`) dans `fn on_document_changed(&mut self, ctx, state: DocState)` ;
    y ajouter à la fin : s'il reste des zones, les effacer et afficher `security.redact.status.cleared`
    (une rotation, une suppression de page ou une annulation rendent les zones fausses ; règle simple et sûre) ;
  - `Response::Redacted { state, report }` → `self.tools.running = None; applying = false; marks.clear();`
    `on_document_changed(ctx, state)`, puis état `security.redact.status.done` (ou `…done_unprotected` si
    `report.encryption_removed`) ; si `last_query` est défini, envoyer `RedactSearch { verify: true }` ;
  - `Response::RedactMatches { query, verify, marks }` → si `verify` : rien quand la liste est vide, sinon
    zones proposées + état `security.redact.status.remaining` ; sinon ajout des zones (sans doublon exact),
    saut à la page de la première, état `…status.marked` ou `…status.none`, `last_query = Some(..)` ;
  - `Response::OperationFailed` (`:489-492`) : si `self.redact.applying`, le remettre à faux et vider
    `self.tools.running`. Les zones sont conservées pour réessayer en mode « page en image ».
- `error_key` (`:1924-1936`) : `EngineError::RedactionIncomplete => "error.redact_incomplete"`, et la variante
  est ajoutée à la liste du test `every_error_key_exists_in_both_catalogs` (`:1944-1954`).
- `eframe::App::ui` (`:1875-1884`) : `self.redact_dialogs(&ctx);` après `self.protect_dialog(&ctx);`.

### 5.7 Boîtes de dialogue (toutes via `dialog::show`, `[desk]/dialog.rs:72-169`, D020)

| Boîte | `Spec` | Corps | Pied (premier ajouté = à droite) |
|---|---|---|---|
| Rechercher (`search_open`) | id `redact-search`, icône `glyph::FIND`, largeur 460 | `dialog::field_label` + champ (focus comme `password_dialog`, `[desk]/app.rs:1612-1619`), case `find.match_case`, note `security.redact.search.note` | `dialog::primary` « Marquer les occurrences » (ou Entrée) → `Request::RedactSearch { verify: false }` ; `dialog::secondary` « Annuler » (ou Échap) |
| Confirmer (`confirm_open`) | id `redact-confirm`, icône `glyph::REDACT`, `.warning()`, largeur 520, sous-titre « {count} zone(s) sur {pages} page(s) » | texte `…confirm.text`, cases `…confirm.rasterize` et `…confirm.clean`, note `…confirm.note` | `dialog::danger` « Caviarder » (ou Ctrl+Entrée, jamais Entrée seule) ; `dialog::secondary` « Annuler » |
| Zones non appliquées (`save_prompt`) | id `redact-unapplied`, icône `glyph::WARNING`, `.warning()`, largeur 480 | texte `…unapplied.text` | `dialog::primary` « Appliquer… » ; `dialog::secondary` « Enregistrer sans caviarder » ; `dialog::secondary` « Annuler » |

Ctrl+Entrée se lit sur l'événement lui-même, `ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))`
(`[egui]/input_state/mod.rs:691-724`), et non sur `i.modifiers` (état en fin d'image) comme le fait
`note_dialog` : sinon le raccourci est perdu quand Ctrl+Maj+B et Ctrl+Entrée arrivent dans la même image
(scénario S3, §8.5).

Validation de la confirmation : `applying = true`, `self.tools.running = Some("security.redact.running")`
(affiché avec un indicateur d'activité par la barre d'état, `[desk]/shell.rs:1167-1171`), puis
`Request::Redact { marks: self.redact.marks.clone(), options: RedactOptions { rasterize_pages, clean_hidden_data, ..Default::default() } }`.

### 5.8 Enregistrement avec des zones non appliquées

Au début de `save_document` (`[desk]/app.rs:1560`) :

```rust
if !self.redact.marks.is_empty() && !std::mem::take(&mut self.redact.save_confirmed) {
    self.redact.save_prompt = Some(ask_path);
    return;
}
```

« Appliquer… » → `save_prompt = None`, `self.after_save = None`, `confirm_open = true`. « Enregistrer sans
caviarder » → `save_confirmed = true` puis `self.save_document(ask_path)`. « Annuler » → `save_prompt = None`,
`self.after_save = None`. Le garde est dans `save_document` et non dans `execute` : la boîte « modifications non
enregistrées » appelle `save_document` directement (`:1526-1530`).

### 5.9 Onglets, annulation, accessibilité

- Onglets : l'état suit le document (§5.4). Un onglet suspendu garde ses zones.
- Annulation : Ctrl+Z après application restaure le document d'avant ; les zones ne reviennent pas.
- Clavier seul : `B` ou la palette, « Caviarder un texte… », Ctrl+Maj+B, Ctrl+Entrée : parcours complet sans
  souris. La barre porte le nombre de zones en texte (lisible par AccessKit) ; les zones dessinées au pinceau
  n'ont pas de nœud d'accessibilité propre (limite notée).

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Nouvelle sous-commande (anglais, JSON, D009), à ajouter à l'enum `Command` (`:22-265`) et à `run` (`:267`) :

```rust
/// Irreversibly redact rectangles and/or every occurrence of a text, then save to --out.
Redact {
    file: PathBuf,
    #[arg(long)] out: PathBuf,
    /// Area "PAGE:LEFT,BOTTOM,RIGHT,TOP" in PDF points (page coordinates, as printed by `search`). Repeatable.
    #[arg(long = "rect")] rects: Vec<String>,
    /// Redact every occurrence of this text. Repeatable.
    #[arg(long = "text")] texts: Vec<String>,
    #[arg(long)] match_case: bool,
    /// Replace every redacted page by an image of itself (no text left on these pages).
    #[arg(long)] rasterize: bool,
    /// Keep metadata and the structure tree (removed by default).
    #[arg(long)] keep_hidden_data: bool,
    #[arg(long, default_value_t = 200.0)] dpi: f32,
    #[arg(long)] password: Option<String>,
}
```

Déroulé : `engine.open` → zones = `--rect` analysés (`fn parse_rect_spec(spec: &str) -> Result<RedactMark, EngineError>`,
page en base 1, cinq nombres sinon `Other("invalid rectangle: …")`) + `doc.redact_marks_for_text` pour chaque
`--text` → aucune zone : `Other("nothing to redact: no rectangle given and no occurrence found")` →
`doc.redact(&marks, &options)?` → pour chaque `--text`, `doc.search(text, match_case)?.len()` : si la somme
n'est pas nulle, `Err(EngineError::RedactionIncomplete)` **avant** toute écriture → `doc.save_as(&out)?`.

Sortie :

```json
{ "out": "out.pdf", "marks": 3, "pages": 2, "objects_removed": 5, "annotations_removed": 1,
  "fields_removed": 0, "pages_rasterized": 0, "hidden_data_cleaned": true,
  "encryption_removed": false, "remaining_matches": 0 }
```

Erreur : `{"error":"redaction_incomplete","message":"…"}` sur stderr, code de sortie 1 (bras `_` de `main`,
`:663-668`), aucun fichier écrit. Test unitaire `parses_rect_specifications` à côté de
`parses_page_specifications` (`:677-689`). `sunipdf-cli commands` liste les trois nouveaux identifiants sans
autre code (`:268-277`).

---

## 7. i18n

Toutes les clés vont dans `assets/i18n/fr-FR.json` **et** `en-US.json`. Préfixes : `menu.security.*` (registre),
`shortcut.*`, `security.redact.*` (préfixe `security.*` réservé par `docs/I18N.md`), `error.*`.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.security.redact` | Caviarder des zones | Redact Areas |
| `menu.security.redact_search` | Caviarder un texte… | Find Text to Redact… |
| `menu.security.redact_apply` | Appliquer le caviardage… | Apply Redactions… |
| `shortcut.tool_redact` | B | B |
| `shortcut.redact_apply` | Ctrl+Maj+B | Ctrl+Shift+B |
| `security.redact.bar.count` | {count} zone(s) à caviarder sur {pages} page(s) | {count} area(s) to redact on {pages} page(s) |
| `security.redact.bar.empty` | Tracez un rectangle sur le contenu à caviarder | Drag a rectangle over the content to redact |
| `security.redact.bar.search` | Rechercher un texte… | Find text… |
| `security.redact.bar.clear` | Tout effacer | Clear all |
| `security.redact.bar.apply` | Appliquer… | Apply… |
| `security.redact.bar.hint` | Cliquez sur une zone pour la retirer. Rien n'est supprimé tant que le caviardage n'est pas appliqué. | Click an area to remove it. Nothing is deleted until the redactions are applied. |
| `security.redact.search.title` | Caviarder un texte | Redact Text |
| `security.redact.search.subtitle` | Chaque occurrence trouvée devient une zone, que vous pouvez retirer avant d'appliquer. | Every occurrence found becomes an area that you can remove before applying. |
| `security.redact.search.label` | Texte à caviarder | Text to redact |
| `security.redact.search.note` | Seul le texte reconnu par le document est trouvé : le texte d'une image numérisée sans OCR ne l'est pas. | Only text the document recognises is found: text inside a scanned image without OCR is not. |
| `security.redact.search.ok` | Marquer les occurrences | Mark occurrences |
| `security.redact.confirm.title` | Appliquer le caviardage ? | Apply Redactions? |
| `security.redact.confirm.subtitle` | {count} zone(s) sur {pages} page(s) | {count} area(s) on {pages} page(s) |
| `security.redact.confirm.text` | Le texte, les images, les tracés et les annotations situés sous les zones seront supprimés du document, pas seulement masqués. Le reste de chaque élément touché est conservé sous forme d'image. | Text, images, drawings and annotations under the areas will be removed from the document, not just covered. The rest of each affected element is kept as an image. |
| `security.redact.confirm.rasterize` | Convertir entièrement les pages caviardées en image (sécurité maximale : leur texte ne sera plus sélectionnable) | Convert redacted pages entirely to images (maximum safety: their text will no longer be selectable) |
| `security.redact.confirm.clean` | Supprimer aussi les informations cachées (métadonnées, balisage d'accessibilité) | Also remove hidden information (metadata, accessibility tags) |
| `security.redact.confirm.note` | Annulable avec Ctrl+Z tant que le document reste ouvert. Le fichier n'est modifié qu'à l'enregistrement. Signets et pièces jointes ne sont pas modifiés. | Can be undone with Ctrl+Z while the document stays open. The file only changes when you save. Bookmarks and attachments are not changed. |
| `security.redact.confirm.ok` | Caviarder | Redact |
| `security.redact.unapplied.title` | Caviardage non appliqué | Redactions Not Applied |
| `security.redact.unapplied.text` | {count} zone(s) sont marquées mais pas appliquées. Le fichier enregistré contiendrait encore le contenu marqué. | {count} area(s) are marked but not applied. The saved file would still contain the marked content. |
| `security.redact.unapplied.apply` | Appliquer… | Apply… |
| `security.redact.unapplied.save` | Enregistrer sans caviarder | Save Without Redacting |
| `security.redact.running` | Caviardage en cours… | Redacting… |
| `security.redact.status.done` | Caviardage appliqué : {marks} zone(s), {objects} élément(s) et {annotations} annotation(s) supprimés. Enregistrez pour l'inscrire dans le fichier. | Redactions applied: {marks} area(s), {objects} element(s) and {annotations} annotation(s) removed. Save to write them to the file. |
| `security.redact.status.done_unprotected` | Caviardage appliqué : {marks} zone(s), {objects} élément(s) et {annotations} annotation(s) supprimés. La protection par mot de passe a été retirée : réappliquez-la avant d'enregistrer. | Redactions applied: {marks} area(s), {objects} element(s) and {annotations} annotation(s) removed. Password protection was removed: apply it again before saving. |
| `security.redact.status.marked` | {count} occurrence(s) marquée(s) sur {pages} page(s) | {count} occurrence(s) marked on {pages} page(s) |
| `security.redact.status.none` | Aucune occurrence à marquer | No occurrence to mark |
| `security.redact.status.remaining` | Attention : {count} occurrence(s) de « {query} » subsistent dans le document. | Warning: {count} occurrence(s) of "{query}" remain in the document. |
| `security.redact.status.cleared` | Zones de caviardage effacées : le document a été modifié. | Redaction areas cleared: the document was modified. |
| `security.redact.status.removed` | Zone retirée | Area removed |
| `error.redact_incomplete` | Le caviardage n'a pas pu être garanti sur ce document : rien n'a été modifié. Réessayez en cochant « Convertir entièrement les pages caviardées en image ». | Redaction could not be guaranteed for this document: nothing was changed. Try again with "Convert redacted pages entirely to images" checked. |
| `error.redact_no_marks` | Aucune zone à caviarder : tracez un rectangle ou recherchez un texte. | Nothing to redact: drag a rectangle or search for a text. |

Valeur modifiée : `home.card.security.desc` → « Mot de passe · Permissions · Métadonnées · Caviardage » /
« Password · Permissions · Metadata · Redaction ».

Réutilisées : `find.match_case`, `dialog.password.cancel`. Dans le JSON en-US, les guillemets droits de
`…status.remaining` et `error.redact_incomplete` s'échappent (`\"`). Les jeux de paramètres `{…}` sont
identiques dans les deux langues (test `placeholders_match_between_languages` de `sunipdf-i18n`). Le test
`ui_keys_used_by_app_exist` ne lit que `app.rs` et `main.rs` (`[desk]/app.rs:1964-1981`) : y ajouter
`include_str!("redact.rs")`.

---

## 8. Tests

### 8.1 Fixtures (`scripts/make_fixtures.py`, puis `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`)

Ajouter l'utilitaire et deux générateurs (appelés dans `__main__`, `:84-88`) :

```python
def stream_with(dict_body, content):
    return b"<< " + dict_body + b" /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream"

def redact_fixture(path):
    content = (b"BT /F1 12 Tf 72 720 Td (Public header line) Tj ET\n"
               b"BT /F1 12 Tf 72 680 Td (Name: SECRET-ALPHA-7731 end of line) Tj ET\n"
               b"BT /F1 12 Tf 72 640 Td (Keep this paragraph KEEPME-42) Tj ET\n"
               b"1 0 0 rg 72 500 200 60 re f\n"
               b"q 128 0 0 128 350 480 cm /Im1 Do Q\n"
               b"BT 3 Tr /F1 10 Tf 360 590 Td (HIDDEN-OCR-5512) Tj ET\n")
    # Image 64x64 RVB brute : lignes 4..27 = marqueur ASCII (moitie haute), le reste vert uni.
    marker = b"IMGSECRET-9F3A--"                      # 16 octets ; 12 x 16 = 192 = 64 pixels x 3
    rows = [bytes([0, 160, 0]) * 64 for _ in range(64)]
    for r in range(4, 28):
        rows[r] = marker * 12
    thumb = b"THUMB-SECRET-01!" * 4                    # 64 octets = 8 x 8 en niveaux de gris
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [10 0 R 11 0 R] /DA (/Helv 10 Tf 0 g)"
        b" /DR << /Font << /Helv 5 0 R >> >> >> /StructTreeRoot 14 0 R /MarkInfo << /Marked true >> >>",   # 1
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",                                                      # 2
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R"
        b" /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >>"
        b" /Annots [7 0 R 8 0 R 9 0 R 10 0 R 11 0 R] /Thumb 12 0 R"
        b" /PieceInfo << /SuniTest << /Private (PIECE-SECRET-02) >> >> >>",                                # 3
        stream(content),                                                                                   # 4
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",                                         # 5
        stream_with(b"/Type /XObject /Subtype /Image /Width 64 /Height 64 /ColorSpace /DeviceRGB"
                    b" /BitsPerComponent 8", b"".join(rows)),                                             # 6
        b"<< /Type /Annot /Subtype /Text /Rect [150 676 170 696] /Contents (NOTE-SECRET-88) /Popup 8 0 R >>",  # 7
        b"<< /Type /Annot /Subtype /Popup /Rect [400 700 580 780] /Parent 7 0 R /Open false >>",           # 8
        b"<< /Type /Annot /Subtype /Link /Rect [72 716 200 732] /Border [0 0 0]"
        b" /A << /S /URI /URI (https://example.invalid/KEEP-LINK) >> >>",                                  # 9
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (ssn) /V (FIELD-SECRET-314)"
        b" /Rect [72 440 272 460] /P 3 0 R /F 4 /DA (/Helv 10 Tf 0 g) >>",                                 # 10
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (city) /V (KEEP-FIELD-9)"
        b" /Rect [300 440 500 460] /P 3 0 R /F 4 /DA (/Helv 10 Tf 0 g) >>",                                # 11
        stream_with(b"/Width 8 /Height 8 /ColorSpace /DeviceGray /BitsPerComponent 8", thumb),             # 12
        b"<< /Title (Redaction fixture) /Subject (META-SECRET-55) >>",                                     # 13
        b"<< /Type /StructTreeRoot /K 15 0 R >>",                                                          # 14
        b"<< /Type /StructElem /S /Figure /P 14 0 R /Pg 3 0 R /Alt (ALT-SECRET-66) >>",                    # 15
        stream(b"% ORPHAN-SECRET-77\n"),                                                                   # 16 : orphelin
    ]
    open(path, "wb").write(build(objs, trailer_extra=b"/Info 13 0 R"))

def incremental_fixture(path):
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        stream(b"BT /F1 12 Tf 72 700 Td (OLDREV-SECRET-01) Tj ET\n"),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    data = bytearray(build(objs))
    prev = int(data[data.rfind(b"startxref") + 9:data.rfind(b"%%EOF")].strip())
    offset = len(data)
    data += b"4 0 obj\n" + stream(b"BT /F1 12 Tf 72 700 Td (Current revision) Tj ET\n") + b"\nendobj\n"
    xref = len(data)
    data += b"xref\n4 1\n" + f"{offset:010d} 00000 n \n".encode()
    data += f"trailer\n<< /Size 6 /Root 1 0 R /Prev {prev} >>\nstartxref\n{xref}\n%%EOF\n".encode()
    open(path, "wb").write(bytes(data))
```

Fichiers produits : `redact.pdf`, `redact-incremental.pdf`. Repères de la page de `redact.pdf` (612 × 792 pt) :
texte secret sur la ligne y = 680 (environ x 110 à 236) ; rectangle rouge (72..272, 500..560) ; image
(350..478, 480..608) dont la bande marquée occupe y 552 à 600 ; texte invisible en (360, 590).
**Ces fixtures sont écrites à la main et n'ont pas été ouvertes par PDFium** : si PDFium refuse un détail,
corriger le générateur, pas les assertions.

### 8.2 Détecteur d'octets (dans `crates/sunipdf-core/tests/engine.rs`)

`lopdf` est une dépendance normale du crate (`crates/sunipdf-core/Cargo.toml`) : utilisable dans les tests
d'intégration sans rien ajouter.

```rust
/// Cherche `needle` dans les octets bruts, puis dans chaque objet relu par lopdf : chaînes (y compris
/// hexadécimales, déjà décodées par l'analyseur), noms, flux décompressés, et chaînes affichées par les
/// opérateurs de texte des flux de contenu (morceaux d'un tableau TJ concaténés). ASCII et UTF-16BE.
fn pdf_contains(bytes: &[u8], needle: &str) -> bool;
fn count_occurrences(bytes: &[u8], needle: &[u8]) -> usize;
```

Indispensable : PDFium réécrit le texte des flux régénérés sous une forme qui n'est pas forcément la chaîne
littérale d'origine **[H]** ; une simple recherche d'octets ASCII pourrait conclure à tort à l'absence. Le
détecteur décode donc les flux (`Stream::decompressed_content`, `lopdf::content::Content::decode`) et il est
lui-même validé par un contrôle positif (T1, étape 2).

### 8.3 Tests d'intégration (PDFium requis, style existant : `engine()`, `fixture()`, `:6-47`)

| Test | Ce qui est vérifié |
|---|---|
| **T1** `redaction_removes_text_from_extraction_bytes_and_pixels` | 1. *Contrôles positifs* : `pdf_contains(fixture, n)` vrai pour les dix marqueurs (`SECRET-ALPHA-7731`, `HIDDEN-OCR-5512`, `IMGSECRET-9F3A`, `NOTE-SECRET-88`, `FIELD-SECRET-314`, `THUMB-SECRET-01`, `PIECE-SECRET-02`, `ORPHAN-SECRET-77`, `META-SECRET-55`, `ALT-SECRET-66`). 2. *Contrôle du détecteur* : ouvrir, `add_shape` (régénère le flux), `snapshot()` → `pdf_contains(.., "SECRET-ALPHA-7731")` encore vrai. 3. Rouvrir ; `redact_marks_for_text("SECRET-ALPHA-7731", true)` → 1 zone page 0 ; `redact(.., &RedactOptions::default())` : `pages == 1`, `objects_removed >= 1`, `annotations_removed == 2` (note + popup), `pages_rasterized == 0`, `hidden_data_cleaned`. 4. Extraction : `page_text(0)` ne contient plus le secret, contient « Public header line » et « KEEPME-42 » ; `search("SECRET-ALPHA", false)` vide. 5. Pixels (`render_page(0, 1.0)`) : tout pixel à plus de 2 px à l'intérieur de la zone est `(0, 0, 0)` ; à gauche de la zone sur la même ligne (x 72..105), plus de 20 pixels sombres (« Name: » conservé en image) ; `(400, 100)` blanc. 6. Octets, sur le fichier écrit par `save_as` **et** sur `snapshot()` : absents `SECRET-ALPHA-7731`, `NOTE-SECRET-88`, `THUMB-SECRET-01`, `PIECE-SECRET-02`, `ORPHAN-SECRET-77`, `META-SECRET-55`, `ALT-SECRET-66` ; présents `KEEP-LINK`, `KEEP-FIELD-9`, `FIELD-SECRET-314`, `IMGSECRET-9F3A` (non marqués : preuve de précision) ; `count_occurrences(.., b"%%EOF") == 1`. 7. Réouverture du fichier : mêmes assertions de texte |
| **T2** `redaction_blacks_out_image_path_and_hidden_text` | Zones : moitié haute de l'image (350, 544, 478, 608) et moitié gauche du rectangle rouge (72, 500, 172, 560). `objects_removed >= 3` ; `page_text(0)` sans `HIDDEN-OCR-5512` ; plus aucun objet `Path` aux limites (72..272, 500..560) dans `page_objects(0)`. Pixels : `(414, 792−576)` noir à 24 près (vignette JPEG), `(414, 792−512)` vert (g > 120, r et b < 80), `(120, 792−530)` noir exact, `(220, 792−530)` rouge. Octets : `IMGSECRET-9F3A` et `HIDDEN-OCR-5512` absents |
| **T3** `redaction_removes_form_field_under_mark` | Zone (72, 440, 272, 460) : `fields_removed == 1` ; `FIELD-SECRET-314` absent, `KEEP-FIELD-9` présent ; `annotations(0)` n'a plus de `Widget` à cet emplacement |
| **T4** `redaction_rasterize_mode_leaves_only_pixels` | `rasterize_pages: true`, zone dans le blanc (500, 100, 520, 120) : `pages_rasterized == 1` ; `page_text(0)` vide ; `page_objects(0)` = un seul objet `Image` ; `annotations(0)` vide ; octets : aucun des dix marqueurs, ni `KEEP-LINK`, ni `KEEP-FIELD-9` ; pixels : zone noire, rectangle rouge encore rouge en `(172, 792−530)` |
| **T5** `redaction_respects_page_rotation` | `two-pages.pdf`, `set_page_rotation(0, Rotation::R90)` ; zone = `redact_marks_for_text("SuniPDF page 1", true)` ; `d = page_rect_to_display(0, zone)` puis `display_rect_to_page(0, d)` ≈ zone (± 0,01) ; après `redact` : `page_rotation(0) == R90`, texte absent, rendu 792 × 612 noir au centre de `d` (`x = (d.left + d.right) / 2`, `y_haut = 612 − (d.bottom + d.top) / 2`), rectangle bleu de la fixture encore bleu à sa position convertie |
| **T6** `redaction_drops_previous_revisions` | `redact-incremental.pdf` : le fichier d'entrée contient `OLDREV-SECRET-01` (contrôle positif) ; `page_text(0)` = « Current revision » ; après une zone quelconque dans le blanc, le fichier enregistré ne le contient plus et n'a qu'un `%%EOF` |
| **T7** `redaction_of_encrypted_document_and_errors` | `security::protect` (AES-256, mot de passe utilisateur) → ouverture → `redact` : `encryption_removed`, fichier enregistré lisible **sans** mot de passe, texte absent. Erreurs : zones vides → `Err` ; page 9 → `PageOutOfRange(9)` ; rectangle dégénéré ou `NaN` → `Err`, document inchangé (`page_text` identique) ; `EngineError::RedactionIncomplete.code() == "redaction_incomplete"` |

### 8.4 Tests unitaires sans PDFium (`#[cfg(test)]` dans `redact.rs`)

| Test | Vérifie |
|---|---|
| **U1** `display_page_mapping_round_trips` | les 4 rotations avec une boîte à origine non nulle : aller-retour identité, et les coins attendus de la table §3.3 |
| **U2** `mark_pixels_round_outwards` | le rectangle de pixels d'une zone contient toujours la zone (floor / ceil), y compris sur les bords de l'image |
| **U3** `sanitize_deletes_resources_no_page_draws` | document lopdf construit en mémoire (`dictionary!`, `add_object`, `[lopdf]/creator.rs:21`) : deux pages partageant un `/Resources` qui liste `/Im1` et `/Im2` ; le contenu des deux pages ne dessine que `/Im2` → après `sanitize`, l'objet de `/Im1` n'existe plus, celui de `/Im2` existe ; variante où la page 2 dessine `/Im1` → conservé ; variante où le contenu de la page 2 est indécodable → `RedactionIncomplete` |
| **U4** `sanitize_removes_popups_replies_and_terminal_fields` | note + popup + réponse `/IRT` ; champ terminal avec deux widgets sur deux pages ; champ non terminal parent d'un autre champ conservé ; `/XFA` retiré |

### 8.5 Scénario manuel de capture (`scripts/dev-screenshot.ps1`)

Coordonnées de `-Drag` : relatives à la fenêtre, en points logiques. Celles-ci sont **estimées** pour la
fenêtre par défaut 1280 × 780 avec le panneau d'outils et les miniatures ouverts (page ajustée à la largeur) ;
faire d'abord S0 puis ajuster.

```powershell
$f = "crates\sunipdf-core\tests\fixtures\redact.pdf"
.\scripts\dev-screenshot.ps1 -Out s0-ouvert.png   -Pdf $f -Commands 'tool.redact'
.\scripts\dev-screenshot.ps1 -Out s1-zones.png    -Pdf $f -Commands 'tool.redact' -Drag '575,220,745,248;882,330,1043,411'
.\scripts\dev-screenshot.ps1 -Out s2-confirme.png -Pdf $f -Commands 'tool.redact' -Drag '575,220,745,248' -KeysAfter '^+b'
.\scripts\dev-screenshot.ps1 -Out s3-applique.png -Pdf $f -Commands 'tool.redact' -Drag '575,220,745,248' -KeysAfter '^+b^{ENTER}'
.\scripts\dev-screenshot.ps1 -Out s4-recherche.png -Pdf $f -Commands 'security.redact_search'
```

À contrôler : S0 l'outil est actif dans la palette et la barre « Caviardage » invite à tracer ; S1 deux zones
translucides à liseré rouge, la barre indique « 2 zone(s)… » ; S2 boîte de confirmation (icône d'avertissement,
bouton rouge « Caviarder », deux cases) ; S3 rectangle noir opaque, « Name: » encore lisible, barre d'état avec
le bilan, titre marqué modifié ; S4 boîte de recherche avec le champ actif. Les deux touches de S3 peuvent
arriver dans la même image : `handle_input` ouvre la boîte avant que `redact_dialogs` ne lise Ctrl+Entrée
(ordre de `[desk]/app.rs:1764-1765` puis `:1875-1884`).

### 8.6 Commandes de validation (à exécuter par l'intégrateur, jamais annoncées sans résultat réel)

`cargo test -p sunipdf-core -j 2`, `cargo test -p sunipdf-i18n -p sunipdf-desktop -p sunipdf-cli -j 2`,
`cargo clippy --workspace -j 2 -- -D warnings`, `cargo fmt --check`, `.\scripts\check-i18n.ps1`.

---

## 9. Nouvelles dépendances

**Aucune.** `pdfium-render` 0.9.4 (fonction `image` activée par défaut), `lopdf` 0.45, `image` 0.25 (codec
`jpeg` déjà activé dans le `Cargo.toml` racine) sont déjà des dépendances de `sunipdf-core`. `zeroize` n'est pas
nécessaire : la tranche ne manipule aucun mot de passe nouveau.

---

## 10. Risques, pièges, ordre d'implémentation, estimation

### 10.1 Risques et pièges connus

| Risque | Parade |
|---|---|
| Table de rotation (§3.3) ou remise à zéro temporaire de `/Rotate` contredites par PDFium | T5. Plan B : ne pas toucher `/Rotate`, convertir les rectangles avec la table et faire pivoter les vignettes (`image::imageops::rotate90/180/270`) |
| Le rendu « sans champs » passe par `FPDF_RenderPageBitmapWithMatrix` : décalage possible d'un pixel | noir arrondi vers l'extérieur ; assertions de pixels à 2 px du bord |
| Limites de texte PDFium plus petites que l'encre (métriques de police fausses) | vérification C → `RedactionIncomplete`, document restauré |
| lopdf refuse en décodage strict un flux que PDFium affiche | `RedactionIncomplete` ; le mode « page en image » ne laisse que du contenu écrit par PDFium |
| Formulaire XObject conservé sans `/Resources` propre, police Type 3 ou motif s'appuyant sur les ressources de la page | noms des formulaires sans `/Resources` collectés (B3) ; les autres cas, rares, peuvent casser un rendu : risque d'affichage, pas de fuite ; à surveiller sur le corpus |
| `prune_objects` et `delete_object` sont quadratiques (`refs.contains` sur un `Vec`, `[lopdf]/document.rs:241`, `[lopdf]/processor.rs:70`) | acceptable jusqu'à quelques dizaines de milliers d'objets ; commentaire `ponytail:` ; si lent, accessibilité maison avec un `BTreeSet` |
| Mémoire : rendu d'une page à 200 ppp + plusieurs copies du document | plafond `RASTER_MAX_PIXELS` ; même ordre de grandeur que `BytesOp` aujourd'hui |
| `FPDFPage_GenerateContent` altère le rendu de pages complexes (risque déjà accepté par D019) | les vignettes préservent l'aspect des objets retirés ; mode image en repli |
| Zones en espace d'affichage côté interface, en espace de page côté moteur | conversion à un seul endroit (worker) ; commentaires sur les champs ; CLI en espace de page uniquement |
| Zones oubliées avant d'enregistrer | boîte « zones non appliquées » (§5.8) |
| Conflits de fusion avec les tranches parallèles (`Tool::ALL`, `Command`, `ToolTab` ou hub, `document_area`, raccourcis `K` / `Ctrl+B` / `S`) | §5.2, §5.5, §5.6 ; faire atterrir `tool-hub` et `continuous-view` avant l'étape 7 si elles sont prêtes |
| Texte recherché journalisé | ne journaliser que des nombres |

### 10.2 Étapes (chacune compile et passe ses tests)

1. `error.rs` : `RedactionIncomplete` + clé `error.redact_incomplete` + `error_key` + liste du test.
2. `redact.rs` : types, table de conversion, `display_rect_to_page`, `page_rect_to_display`,
   `redact_marks_for_text` ; exports ; U1.
3. `sanitize` et ses aides (B0 à B6), `strip_metadata_in` ; U3, U4.
4. Fixtures ; `redact_page_objects` et `redact` (phases A et C) ; U2, T1 à T7.
5. CLI `redact` + test d'analyse des rectangles.
6. Worker : `edit_with`, `Request::Redact`, `Request::RedactSearch`, réponses.
7. Registre (3 commandes), clés i18n, icône, listes d'outils ; tests du registre et des catalogues.
8. Bureau : `redact.rs` (état, dessin, barre, trois boîtes), crochets dans `app.rs`, `annotate.rs`,
   `editor.rs`, `tabs.rs` ; garde d'enregistrement.
9. Captures S0 à S4, documentation (§11).

### 10.3 Estimation

**L** : moteur ≈ 450 lignes, tests et fixtures ≈ 400, bureau ≈ 350, CLI ≈ 70. L'étape 4 est la plus incertaine
(premier contact réel avec PDFium : table de rotation, alignement des pixels, fixtures écrites à la main).

### 10.4 Questions ouvertes

1. Option « informations cachées » cochée par défaut : elle retire le balisage d'accessibilité de tout le
   document. À confirmer avec l'architecte et l'agent forms-accessibility (alternative : décochée par défaut
   et risque documenté).
2. Module dans `sunipdf-core` plutôt que crate `sunipdf-security` : validation de l'architecte.
3. Raccourcis `B` et Ctrl+Maj+B : arbitrage global avec les autres tranches.
4. Après caviardage d'un document chiffré, faut-il proposer immédiatement « Protéger » ?
5. Couleur de remplissage et texte en surimpression (« CAVIARDÉ ») : non prévus en v1.
6. Faut-il refuser le caviardage quand les permissions du document interdisent la modification ? (Aucun outil
   d'édition actuel ne le vérifie.)

---

## 11. Documentation à mettre à jour

| Fichier | Contenu |
|---|---|
| `docs/SECURITY_MODEL.md` (**à créer**, périmètre security-signatures) | méthode A/B/C, tableau des garanties (§2.4), limites (§2.5), hypothèses [H], ce que les tests prouvent et ne prouvent pas |
| `docs/DECISIONS.md` | nouvelle décision (prochain numéro libre, D021 au moment de la rédaction) : caviardage par retrait d'objets + vignettes + assainissement lopdf ; pas de découpe de texte ; module dans `sunipdf-core` |
| `docs/KNOWN_ISSUES.md` | limites 1 à 10 de §2.5 ; « supprimer un objet avec l'outil d'édition n'est pas un caviardage » ; les outils existants confondent espace d'affichage et espace de page sur les pages pivotées (seul le caviardage convertit) |
| `docs/FEATURE_MATRIX.md` | ligne « Caviardage irréversible » (section E) : état réel après tests |
| `docs/ROADMAP.md`, `docs/PROJECT_STATUS.md`, `docs/NEXT_TASKS.md` | phase 5 : caviardage livré ; restes : couleur et surimpression, marques `/Redact` persistantes, découpe fine du texte, nettoyage des pièces jointes et du JavaScript |
| `docs/ARCHITECTURE.md` | module `redact` ; `sunipdf-security` reste « planifié » |
| `docs/I18N.md` | préfixe `security.redact.*` |
| `docs/USER_GUIDE.fr.md`, `docs/USER_GUIDE.en.md` | section Sécurité › Caviarder : marquer, rechercher, appliquer, enregistrer ; encadré des limites ; exemple CLI |
| `CHANGELOG.md`, `README.md`, `CLAUDE.md` (liste des modules et des commandes CLI) | mention de la fonction, seulement une fois les tests verts |
