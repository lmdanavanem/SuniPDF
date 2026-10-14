# Plan d'implémentation — tranche `ocr` (PDFCraft « Scan & OCR »)

Rédigé le 2026-10-14 pour SuniPDF 0.3.0. **Aucun code n'a été écrit et `cargo` n'a pas été exécuté** (consigne : poste à
14 Go, agents concurrents). Tout ce qui est marqué « mesuré » provient de commandes lancées sur ce poste (Annexe B) ;
tout ce qui est marqué « à vérifier » est une hypothèse que l'étape indiquée doit confirmer. Les citations `fichier:ligne`
renvoient aux sources réelles du registre Cargo local (préfixes en Annexe A).

## 0. Résumé des décisions

1. **Moteur de reconnaissance = `Windows.Media.Ocr`** (WinRT, local, fourni par Windows), appelé depuis `sunipdf-core` via la crate
   `windows` 0.62.2, **déjà verrouillée dans `Cargo.lock`** (tirée par `accesskit_windows`, donc déjà compilée dans l'application).
   Aucun binaire, aucun modèle, aucune licence tierce à redistribuer.
2. **Tesseract est écarté de cette tranche** (raisons chiffrées en 2.2), mais l'architecture le permet : un trait `Recognizer`
   sépare « image → mots » de « mots → couche de texte invisible ».
3. **Pas de nouveau crate** : un module `sunipdf-core::ocr` (le texte invisible s'écrit avec les objets de page PDFium, donc avec
   `Document`). Le crate `sunipdf-ocr` prévu par `ARCHITECTURE.md` ne se justifiera qu'avec un second moteur (question ouverte pour l'architecte).
4. **Couche de texte** : un `PdfPageTextObject` Helvetica **par mot**, mode de rendu `Invisible` (`3 Tr`), étiré et positionné pour que sa
   boîte coïncide avec la boîte du mot reconnu ; page rendue à ~300 ppp ; les pages qui ont déjà du texte sont ignorées par défaut.
5. **Exécution** : un état « tâche OCR » dans `worker.rs`, avancé **une page à la fois** par la boucle du thread moteur (comme le
   pré-rendu). Cela donne progression, annulation et réactivité (rendus, recherche) sans second thread PDFium. Une seule entrée
   d'annulation (instantané pris au début) pour toute la tâche.
6. **Le français dépend d'un pack Windows** : sur ce poste **seul `en-GB` est installé** (mesuré). L'interface détecte l'absence,
   désactive le lancement et explique comment installer le pack ; aucune bascule silencieuse vers l'anglais.

## 1. Objectif et comportement visible

**Outil PDFCraft correspondant : « Scan & OCR ».** L'utilisateur ouvre un PDF numérisé (pages = images), lance
*Convertir › Reconnaissance de texte (OCR)…* (panneau d'outils catégories « Convertir » et « Tous les outils », menu Convertir,
palette Ctrl+K ; commande `convert.ocr`), choisit la langue (Français / Anglais), les pages (toutes / courante / sélection de la vue
Organiser) et clique « Lancer l'OCR ». Comme les autres outils, la commande est utilisable sans document (elle ouvre d'abord un fichier : `after_open`).

- Une boîte modale (`dialog::show`) affiche une barre de progression « Page 3 sur 12 · 1 245 mot(s) reconnu(s) » et un bouton
  « Annuler l'OCR ». Le moteur reste réactif (rendu, recherche) entre deux pages.
- À la fin : message d'état vert « OCR terminé : 12 page(s) traitée(s), 3 802 mot(s) reconnu(s). », document marqué modifié,
  **une seule entrée Ctrl+Z** annule tout. L'aspect des pages est **strictement identique** (texte invisible) ; Ctrl+F trouve
  les mots (surbrillance alignée sur le texte numérisé), la sélection de texte et Ctrl+C fonctionnent, `sunipdf-cli text` renvoie le texte.
- Les pages qui contiennent déjà ≥ 20 caractères non blancs sont ignorées (case « Ignorer les pages qui contiennent déjà du texte »,
  cochée par défaut). Décochée, la couche invisible précédente est **remplacée** (relance avec une autre langue) ; le texte visible est conservé.
- Pack de langue absent : encart explicatif dans la boîte (nom du pack, chemin dans Paramètres, commande administrateur
  `Add-WindowsCapability -Online -Name Language.OCR~~~fr-FR~0.0.1.0`), boutons « Ouvrir les paramètres de langue » et
  « Vérifier à nouveau » ; « Lancer l'OCR » grisé tant que la langue choisie manque.
- Composant OCR absent de Windows : message dédié, tout est grisé.
- Annulation : les pages déjà traitées sont **conservées** (une entrée d'annulation) ; message « OCR annulé : N page(s) conservée(s)
  (Ctrl+Z pour annuler). ».
- CLI : `sunipdf-cli ocr fichier.pdf --lang fr --out sortie.pdf` et `sunipdf-cli ocr-languages` (JSON).

Hors de cette tranche (volontairement, voir 2.4) : scanner physique (WIA/TWAIN), export Word/texte structuré, Tesseract,
plusieurs langues par passage, redressement d'image, écriture manuscrite. Pour « image → PDF consultable » : *Créer un PDF à partir
d'images* puis OCR ; un enchaînement en un clic est une suite simple (question ouverte 4).

## 2. Faisabilité

### 2.1 Verdict : **partiel**

- **Oui** pour : OCR hors ligne fr/en là où le pack de langue Windows est installé, couche de texte invisible
  recherchable/sélectionnable, progression, annulation, Ctrl+Z, CLI, tests automatisés (moteur factice déterministe + tests réels
  conditionnels).
- **Partiel** parce que : (a) le **pack français n'est pas installé sur le poste de développement** (seul `en-GB`) : le chemin
  français de bout en bout ne sera vérifiable qu'après installation du pack (action utilisateur, éventuellement administrateur) ;
  (b) **Tesseract n'est pas livré** ; (c) la **précision n'a pas été mesurée sur un corpus** (voir 2.3) ; (d) une seule langue par passage.

### 2.2 Windows.Media.Ocr contre Tesseract embarqué

| Critère | Windows.Media.Ocr (retenu) | Tesseract |
|---|---|---|
| Licence | composant du système, rien à redistribuer ; crate `windows` MIT OR Apache-2.0 | Apache-2.0 (moteur et `tessdata*` : fichiers `LICENSE` de 11 358 o dans `tessdata` et `tessdata_fast`, 10 173 o dans `tessdata_best`) |
| Poids ajouté au MSI/ZIP | 0 | `tessdata_fast` : eng 4 113 088 o + fra 1 130 365 o = **≈ 5,2 Mo** ; `tessdata_best` : 15 400 601 + 3 972 885 = **≈ 19,4 Mo** ; `tessdata` (hérité + LSTM) : 23 466 654 + 14 213 351 = **≈ 37,7 Mo** ; plus `tesseract.exe`/DLL ou bibliothèque statique (non mesuré). Tailles : API GitHub `tesseract-ocr/tessdata*`, interrogée le 2026-10-14. |
| Langues | celles installées dans Windows (fr absent ici) ; **une** langue par passage | `fra+eng` combinables en un passage |
| Construction | `windows` déjà compilée ; ajout de features | Crates `tesseract` 0.15.2, `leptess` 0.14.0, `tesseract-sys` 0.6.3 (toutes MIT) : Tesseract + Leptonica via **vcpkg** sous Windows (README `leptess`, consulté le 2026-10-14) et **clang** pour bindgen. Sur ce poste `cmake`, `clang`, `ninja`, `vcpkg`, `tesseract` sont **introuvables** (mesuré). `tesseract-static` 0.2.1 (Apache-2.0, 2 526 téléchargements) non évalué. Variante sous-processus (`rusty-tesseract` 1.1.10, MIT) : livrer `tesseract.exe` + DLL + `tessdata`. |
| Vitesse (mesurée ici) | **225–255 ms** pour la 1re page (démarrage du moteur), **56–91 ms** ensuite ; page A4 300 ppp, 4 lignes (PowerShell, image synthétique) | non mesurée (non installé) |
| Précision | non mesurée sur corpus (voir 2.3) | non mesurée |
| Hors ligne | traitement par une API WinRT locale — **à confirmer par une capture réseau avant d'écrire « hors ligne » dans la documentation** ; l'installation d'un pack passe par Windows Update, en dehors de SuniPDF | oui |
| Dépendance OS | Windows 10/11 + pack de langue ; ARM64 : même API | aucune |

Conclusion : l'ADR **D021** consigne « Windows.Media.Ocr par défaut, Tesseract reporté » en précisant que le choix s'est fait sur
disponibilité, licence et poids, **pas sur un CER comparé** (Tesseract n'a pas pu être installé ici). Le protocole de mesure CER
est fourni (tests 8.2, commande en 8.3) pour trancher plus tard.

### 2.3 Ce qui a été mesuré sur ce poste (Windows 11 build 26300, `Windows.Media.Ocr.dll` 10.0.26100.9278)

- Langues de reconnaissance installées : **`en-GB` seulement**. `OcrEngine.TryCreateFromLanguage("fr-FR")` renvoie **null** ;
  `IsLanguageSupported(fr-FR)` = False. `TryCreateFromLanguage("en-US")` renvoie un moteur dont `RecognizerLanguage` est **`en-GB`**
  (correspondance par langue) : on affiche donc la langue *réellement* utilisée. Dossier système : `C:\Windows\OCR\en-gb`.
- `MaxImageDimension` = **10 000** ; une image de 10 001 px de large échoue avec `0x80070057` « Image dimensions are too large! ».
- Formats `SoftwareBitmap` acceptés : `Bgra8`, `Rgba8`, `Gray8` (les trois ont reconnu le même texte). `Bgra8` est retenu (format des
  exemples Microsoft) ; `Gray8` = piste d'optimisation mémoire (10.3).
- Image synthétique A4 300 ppp (Arial 58 px), moteur `en-GB` : lignes anglaises quasi exactes ; « 1,250.50 » est **scindé en deux
  mots** (« 1 » et « ,250.50 ») → CER 1/87 = **1,1 %**. Lignes françaises avec le moteur anglais : « L'école est fermée dès l'été. »
  devient « est ferrnée », « à la » devient « Ia » → CER 25/87 = **28,7 %**. *Une seule image, un seul passage : illustration de la
  nécessité du pack français, pas un benchmark.*
- Les `OcrWord.BoundingRect` sont des **boîtes d'encre en pixels de l'image, origine en haut à gauche** (« over » : 33 px de haut,
  « quick » : 54 px) ; `TextAngle` = 0.
- PDFium (build 7881, `third_party/pdfium/win-x64/bin/pdfium.dll`, appels C via ctypes) : un objet texte au mode de rendu 3 est
  sérialisé en `3 Tr` dans le flux régénéré, produit **0 pixel sombre** au rendu (objet visible témoin : 70 pixels), et
  `sunipdf-cli text`/`search` le retrouvent. Des objets-mots séparés sur une même ligne sont extraits **avec des espaces** et
  « quick brown » est trouvé à travers deux objets (CLI sur PDF fabriqués, accents WinAnsi compris : « école », « fermée », « déjà »).
  Les `bounds` d'un objet texte sont les **boîtes d'encre** des glyphes (« Facture » à 10 pt : 0,82 / −0,11 / 33,49 / 7,16), valides
  avant insertion et sans régénération du contenu. Régénérer le contenu à chaque ajout coûte 0,103 s pour 500 objets contre 0,003 s en une fois.

### 2.4 Limites assumées (à reporter dans `KNOWN_ISSUES.md` et le guide)

1. Une langue par passage (pas de document fr+en mélangé) ; pas de détection automatique de la langue.
2. Qualité = celle du moteur Windows et du pack installé ; **aucun CER sur corpus n'est promis**. Les nombres avec séparateurs et les
   mots collés peuvent être scindés (« 1 ,250.50 »).
3. Pas de redressement : au-delà de quelques degrés d'inclinaison, la surbrillance est décalée (`text_angle_deg` est rapporté, pas
   corrigé). Pas de détection d'un contenu tourné sans `/Rotate`. Les pages avec `/Rotate` 90/180/270 sont gérées par la géométrie (3.5).
4. Texte écrit en Helvetica standard étiré à la boîte du mot : surbrillance précise **au mot**, pas au caractère. Les caractères
   hors WinAnsi sont remplacés par `?` ; les ligatures `ﬁ ﬂ ﬀ ﬃ ﬄ` sont développées.
5. Ni PDF/A, ni structure balisée (accessibilité), ni export hOCR/texte. Le texte s'extrait avec la commande existante `text`.
6. Taille du fichier : ≈ +60 octets par mot avant compression (≈ 30 Ko pour 500 mots, **estimation non mesurée**) ; « Optimiser » compresse.
7. Pas de délai maximal par page dans la version de base (risque R7, durcissement 10.3).
8. Éditions de Windows : voir 2.5.

### 2.5 Éditions de Windows sans la capacité OCR

- Le composant `C:\Windows\System32\Windows.Media.Ocr.dll` est présent ici (Windows 11 Pro). Sur une édition ou un SKU où la classe d'exécution
  n'est pas enregistrée, `OcrEngine::AvailableRecognizerLanguages()` échoue (`RoGetActivationFactory`, typiquement
  `REGDB_E_CLASSNOTREG`) : **comportement attendu, non vérifié ici** (aucune édition de ce type disponible). Le moteur renvoie alors
  `EngineError::OcrUnavailable` ; l'interface affiche `ocr.unavailable` et grise le lancement ; la CLI sort avec le code 6 ; les
  tests réels se **sautent** avec un message.
- Composant présent mais **aucun pack de langue** : la liste est vide ; `OcrLanguageMissing(tag)` pour toute langue demandée.
- Ajout d'un pack par l'utilisateur : capacité `Language.OCR~~~<tag>~0.0.1.0` (dépend de `Language.Basic~~~<tag>~0.0.1.0`), exemple
  officiel `fr-FR`. Windows 11 : un utilisateur standard peut ajouter les fonctionnalités de langue depuis Paramètres › Heure et langue ;
  Windows 10 : page Langue des Paramètres (source : learn.microsoft.com, « Language and region Features on Demand », consultée le 2026-10-14).
  Libellé exact de l'option dans Paramètres (« Reconnaissance optique de caractères » en français) : **à vérifier** à la recette.
- Windows Server, éditions N, LTSC : non évaluées.

## 3. API moteur (`sunipdf-core`)

### 3.1 Fichiers touchés

| Fichier | Changement |
|---|---|
| `crates/sunipdf-core/Cargo.toml` | dépendance `windows` 0.62 (features en 9) |
| `crates/sunipdf-core/src/lib.rs` | `pub mod ocr;` + `pub use ocr::{…}` |
| `crates/sunipdf-core/src/error.rs` | 3 variantes d'erreur typées + codes stables |
| `crates/sunipdf-core/src/edit.rs` | `ObjectInfo.invisible` ; `object_at` ignore le texte invisible ; `quad_to_rect` en `pub(crate)` |
| `crates/sunipdf-core/src/command.rs` | `Command::Ocr` + entrée `REGISTRY` |
| `crates/sunipdf-core/src/ocr.rs` (nouveau) | types, trait `Recognizer`, géométrie, `ocr_page`, `ocr_pages` |
| `crates/sunipdf-core/src/ocr/winrt.rs` (nouveau) | `WinRecognizer`, `probe`, init WinRT du thread |
| `scripts/make_fixtures.py` | deux fixtures (8.1) |

### 3.2 Erreurs (`error.rs`)

```rust
#[error("Windows text recognition (Windows.Media.Ocr) is not available on this system")]
OcrUnavailable,
#[error("no Windows OCR language pack installed for \"{0}\"")]
OcrLanguageMissing(String),
#[error("text recognition failed: {0}")]
OcrFailed(String),
```
`code()` : `"ocr_unavailable"`, `"ocr_language_missing"`, `"ocr_failed"`. Conséquences : `apps/sunipdf-desktop/src/app.rs::error_key` (match
exhaustif) reçoit `error.ocr_unavailable`, `error.ocr_language_missing`, `error.ocr_failed` ; la liste du test
`every_error_key_exists_in_both_catalogs` est étendue ; la CLI mappe ces deux premiers codes sur le code de sortie **6**.

### 3.3 Types et signatures publics (`ocr.rs`)

```rust
pub const DEFAULT_OCR_DPI: f32 = 300.0;
pub const MIN_TEXT_CHARS: usize = 20;          // seuil « la page a déjà du texte »

/// Boîte en pixels de l'image analysée, origine en haut à gauche, y vers le bas.
#[derive(Debug, Clone, Copy, PartialEq)] pub struct PixelRect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }
#[derive(Debug, Clone, PartialEq)] pub struct RecognizedWord { pub text: String, pub rect: PixelRect }
#[derive(Debug, Clone, PartialEq, Default)] pub struct RecognizedLine { pub words: Vec<RecognizedWord> }
#[derive(Debug, Clone, PartialEq, Default)] pub struct RecognizedPage { pub lines: Vec<RecognizedLine>, pub text_angle_deg: Option<f32> }

/// Moteur de reconnaissance : image RGBA 8 bits (non prémultipliée) → mots. Aucune dépendance à PDFium.
pub trait Recognizer {
    /// Étiquette BCP-47 de la langue réellement utilisée (ex. « en-GB » pour une demande « en-US »).
    fn language_tag(&self) -> &str;
    /// Plus grand côté d'image accepté, en pixels (10 000 pour Windows.Media.Ocr, mesuré).
    fn max_image_dimension(&self) -> u32;
    fn recognize(&self, rgba: &[u8], width: u32, height: u32) -> Result<RecognizedPage, EngineError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrOptions { pub dpi: f32, pub skip_pages_with_text: bool }   // Default : 300.0, true

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrOutcome { Done, SkippedHasText, NoTextFound }

#[derive(Debug, Clone, PartialEq)]
pub struct OcrPageReport { pub page: usize, pub outcome: OcrOutcome, pub words: usize, pub lines: usize,
                           pub dpi: f32, pub text_angle_deg: Option<f32> }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcrSummary { pub pages: Vec<OcrPageReport>, pub cancelled: bool }
impl OcrSummary { pub fn processed(&self) -> usize; pub fn skipped_text(&self) -> usize;
                  pub fn empty(&self) -> usize; pub fn words(&self) -> usize; }

impl Document<'_> {
    /// Vrai si la page contient au moins `MIN_TEXT_CHARS` caractères non blancs (texte visible OU invisible).
    pub fn page_has_text(&self, page_index: usize) -> Result<bool, EngineError>;

    /// OCR d'une page : rend, reconnaît, puis écrit la couche invisible. Aucune modification du document
    /// avant la fin de la reconnaissance ; en cas d'erreur d'écriture PDFium l'appelant doit restaurer un instantané
    /// (le worker le fait ; `ocr_pages` aussi).
    pub fn ocr_page(&mut self, page_index: usize, recognizer: &dyn Recognizer, options: &OcrOptions)
        -> Result<OcrPageReport, EngineError>;

    /// Boucle atomique (CLI, tests) : instantané au début, `restore` sur erreur. `progress(rapport, faites, total)`
    /// peut renvoyer `ControlFlow::Break(())` pour annuler (les pages déjà faites sont conservées, `cancelled = true`).
    pub fn ocr_pages(&mut self, pages: &[usize], recognizer: &dyn Recognizer, options: &OcrOptions,
        progress: &mut dyn FnMut(&OcrPageReport, usize, usize) -> std::ops::ControlFlow<()>)
        -> Result<OcrSummary, EngineError>;
}
```
Réexports dans `lib.rs` : `pub use ocr::{OcrOptions, OcrOutcome, OcrPageReport, OcrSummary, PixelRect, RecognizedLine, RecognizedPage, RecognizedWord, Recognizer, WinRecognizer, OcrEnvironment, OcrLanguageInfo};`.

Validation d'entrée : `options.dpi` fini et dans `[72, 600]` sinon `EngineError::Other("invalid dpi")` (même style que `export_images`) ; page hors limites → `PageOutOfRange` (via `page_size`, `lib.rs:215`).

### 3.4 Algorithme de `ocr_page` (ordre imposé : reconnaître avant de modifier)

1. `size = self.page_size(i)?` (`lib.rs:215`, taille **affichée** en points). `rot = self.page_rotation(i)?` (`pages.rs:151`).
2. Si `options.skip_pages_with_text && self.page_has_text(i)?` → rapport `SkippedHasText`, rien d'autre.
   `page_has_text` = `self.page_text(i)?` (`text.rs:86`, `FPDFText` via `page.text()?.all()`) puis comptage des caractères non blancs ≥ `MIN_TEXT_CHARS`.
3. Rendu : `scale = render_scale(options.dpi, size.width_pt, size.height_pt, recognizer.max_image_dimension())` (fonction pure, 3.5) puis
   `img = self.render_page(i, scale)?` (`lib.rs:224` : RGBA, fond blanc, annotations et formulaires rendus). `report.dpi = scale * 72.0`.
   Mémoire crête par page A4 300 ppp : 34,8 Mo (RGBA) + copie BGRA + `DataWriter` + `SoftwareBitmap` ≈ 140 Mo (estimation, non mesurée), libérée page après page.
4. `page_result = recognizer.recognize(&img.rgba, img.width, img.height)?` — c'est la seule étape longue ; **le document n'a pas été touché**.
5. Planification (fonction pure, testable sans PDFium) : pour chaque mot, texte assaini (`sanitize_word`, 3.5), rectangle pixels → points
   d'affichage (`kx = size.width_pt / img.width as f32`, `ky = size.height_pt / img.height as f32`), puis `display_to_user(rot, …)` et décalage
   d'origine de page (3.5). Mots vides, de taille nulle ou composés uniquement de `?` ignorés. Plan vide → `NoTextFound`, **fin sans modification**.
6. Écriture (une page PDFium `page = self.page_handle(i)?`, `pages.rs:282`) :
   - jeton de police **avant** d'emprunter la page : `let font = self.inner.fonts_mut().helvetica();` (`fonts.rs:194` ; `PdfFontToken` est `Copy`, `fonts.rs:83-86`) — même ordre que `edit.rs:210-211`.
   - `page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual)` (`page.rs:909`, retourne `()`) : un seul `regenerate_content()` final
     (30× plus rapide, mesuré 2.3). **Piège** : en `Manual`, `Drop` ne régénère pas (`page.rs:998-1006`) — le `regenerate_content()` explicite est obligatoire.
   - si `!options.skip_pages_with_text` : supprimer d'abord les objets texte invisibles existants (couche d'un OCR précédent) : indices collectés via
     `page.objects().iter().enumerate()` + `o.as_text_object().is_some_and(|t| !t.is_visible())` (`object.rs:433`, `text.rs:253`), puis
     `page.objects_mut().remove_object_at_index(i as PdfPageObjectIndex)` du plus grand au plus petit (`objects/common.rs:368`, même appel que `edit.rs:457`).
   - pour chaque mot du plan : `let mut obj = PdfPageTextObject::new(&self.inner, &text, font, PdfPoints::new(10.0))?` (`text.rs:197`) ;
     `obj.set_render_mode(PdfPageTextRenderMode::Invisible)?` (`text.rs:424`, variante `text.rs:73`) ; `place_word(&mut obj, cible, rot)?` (3.5) ;
     `page.objects_mut().add_text_object(obj)?` (`objects/common.rs:127`). Les transformations se font **avant** l'ajout (comme `edit.rs:216`), les
     `bounds()` sont valides avant insertion (mesuré).
   - `page.regenerate_content()?` (`page.rs:942`).
7. Rapport `Done { words, lines }`, `text_angle_deg` recopié.

`ocr_pages` : `let snapshot = self.snapshot()?` (`pages.rs:246`) ; boucle sur `ocr_page` ; sur `Err` → `self.restore(snapshot)` (`pages.rs:266`) puis `Err`.

### 3.5 Géométrie, assainissement, échelle (fonctions pures, tests unitaires sans PDFium)

**Affichage → espace utilisateur non pivoté.** Entrée : taille affichée `(W, H)` en points (celle de `PageSize`), rectangle `(u0,v0)-(u1,v1)`
en points depuis le coin haut-gauche de la page affichée (u vers la droite, v vers le bas). `/Rotate` est horaire. Sortie : `PointRect` (origine
bas-gauche, espace utilisateur sans rotation). Formules (dérivées à la main, coins vérifiés) :

| Rotation | x | y | rotation du texte (sens horaire passé à `rotate_clockwise_degrees`) |
|---|---|---|---|
| 0° | `u` | `H − v` | 0 |
| 90° | `v` | `u` | −90 |
| 180° | `W − u` | `v` | −180 |
| 270° | `H − v` | `W − u` | −270 |

Table de test (page utilisateur 612×792, rectangle affiché u 72..192, v 100..130 ; affichée 612×792 en 0°/180°, 792×612 en 90°/270°) :
0° → x 72..192, y 662..692 ; 90° → x 100..130, y 72..192 ; 180° → x 420..540, y 100..130 ; 270° → x 482..512, y 600..720.
Coins : 90°, affiché (0,0) → utilisateur (0,0) ; 270°, affiché (792,612) → utilisateur (0,0).

**Décalage d'origine** : `stamp.rs` et `add_text` supposent la page en (0,0). Pour l'OCR on ajoute `(ox, oy)` = coin bas-gauche de
`page.boundaries().crop()` (`boundaries.rs:220`, `PdfPageBoundaryBox.bounds`, `boundaries.rs:27-30`), repli `media()` (`boundaries.rs:113`), repli (0,0).
Test : fixture à MediaBox décalée (8.1).

**`place_word`** (objet créé à l'origine, taille 10 pt, boîte d'encre mesurée par `bounds()`) :
`b0 = quad_to_rect(obj.bounds()?)` ; `along/across` = dimensions cible le long/en travers de la ligne de base (permutées pour 90°/270°) ;
ignorer si `b0` ou la cible a une dimension < 0,1 pt ; `obj.translate(-b0.left, -b0.bottom)` ; `obj.scale(along / w0, across / h0)` ;
si rotation ≠ 0 `obj.rotate_clockwise_degrees(-(rot as f32))` ; `b1 = bounds()` ; `obj.translate(cible.left − b1.left, cible.bottom − b1.bottom)`.
Appels : `translate`/`scale`/`rotate_clockwise_degrees` sur `PdfPageTextObject` via `create_transform_setters!` (`text.rs:517`, macro `transform.rs:8`) ;
`bounds()` via l'impl générique `PdfPageObjectCommon` (`object.rs:862-872`). Même schéma que `resize_object` (`edit.rs:284-289`).
Les boîtes d'encre PDFium (mesuré) et Windows étant homogènes, la hauteur apparente de police est quasi constante sur une ligne.

**`render_scale(dpi, w_pt, h_pt, max_dim) -> f32`** = `min(dpi/72, (min(max_dim, MAX_RENDER_SIDE_PX) − 1) / max(w_pt, h_pt))`. Exemples de test : A4 595×842 à 300 ppp →
4,1667 ; A0 2384×3370 avec `max_dim` 10 000 → 2,967 (≈ 214 ppp).

**`sanitize_word`** : développe `\u{FB01}`→`fi`, `\u{FB02}`→`fl`, `\u{FB00}`→`ff`, `\u{FB03}`→`ffi`, `\u{FB04}`→`ffl` ; supprime les caractères de contrôle ; garde
U+0020..U+007E, U+00A0..U+00FF et les 27 caractères cp1252 hors Latin-1 (`€ ‚ ƒ „ … † ‡ ˆ ‰ Š ‹ Œ Ž ‘ ’ “ ” • – — ˜ ™ š › œ ž Ÿ`) ; le reste devient `?`. Justification : les
polices standard PDFium ne couvrent que WinAnsi (`docs/KNOWN_ISSUES.md` A6, test `edit_page_content_text_image_shapes` : « Bonjour éditeur » relu).

### 3.6 Reconnaissance Windows (`ocr/winrt.rs`)

Types : `pub struct OcrLanguageInfo { pub tag: String, pub display_name: String }`, `pub struct OcrEnvironment { pub languages: Vec<OcrLanguageInfo>, pub max_image_dimension: u32 }`,
`pub struct WinRecognizer { engine: OcrEngine, tag: String, max_dim: u32 }` (`OcrEngine` est `Send + Sync`, `Media/Ocr/mod.rs:153-154`).

```rust
/// Langues installées + dimension maximale. Err(OcrUnavailable) si la classe WinRT est absente.
pub fn probe() -> Result<OcrEnvironment, EngineError>;
/// « fr » / « en » (ou une étiquette complète) → étiquette installée correspondante, sinon « fr-FR » / « en-US » (l'erreur sera OcrLanguageMissing).
pub fn resolve_language(env: &OcrEnvironment, wanted: &str) -> String;
impl WinRecognizer { pub fn new(language_tag: &str) -> Result<Self, EngineError>; }
impl Recognizer for WinRecognizer { … }
```

**Threading (point critique).** Toutes les API WinRT s'appellent depuis le **thread moteur** (`sunipdf-engine`, un `std::thread` sans COM). Au premier usage de ce thread,
`init_thread()` appelle `RoInitialize(RO_INIT_MULTITHREADED)` (`Win32/System/WinRT/mod.rs:233`, constante `:2567`, feature `Win32_System_WinRT`), gardé par un
`thread_local! Cell<bool>` ; l'erreur (par ex. `RPC_E_CHANGED_MODE` si un autre code a déjà initialisé le thread en STA) est ignorée ; pas de `RoUninitialize` (le thread vit autant que
le processus ; commentaire `ponytail:`). `OcrEngine` est déclaré agile par les métadonnées WinRT (`Send + Sync` générés, `Media/Ocr/mod.rs:153-154`), donc aucune pompe de messages ne devrait être nécessaire (à confirmer par les tests réels 5 et 6). Le fil principal egui/winit reste en STA et n'appelle jamais l'OCR.
Les tests initialisent aussi leur propre thread via `WinRecognizer::new`. L'attente de `RecognizeAsync` est bloquante sur le thread moteur (`join()`).

**`WinRecognizer::new(tag)`** :
1. `init_thread()`.
2. `OcrEngine::AvailableRecognizerLanguages()` (`Ocr/mod.rs:106`) ; `Err` → `OcrUnavailable`.
3. `Language::CreateLanguage(&HSTRING::from(tag))` (`Globalization/mod.rs:2778`; `HSTRING: From<&str>`, `windows-strings hstring.rs:129`) ; `Err` → `OcrLanguageMissing(tag)`.
4. `OcrEngine::TryCreateFromLanguage(&language)` (`Ocr/mod.rs:123`). Une valeur WinRT nulle devient `Err(Error::empty())` (`windows-core type.rs:39-45`, `windows-result error.rs:82`) → `OcrLanguageMissing(tag)` (mesuré : `fr-FR` absent → null).
5. `engine.RecognizerLanguage()` (`Ocr/mod.rs:92`) puis `LanguageTag()` (`Globalization/mod.rs:2729`) puis `to_string_lossy()` (`hstring.rs:23`) → `tag` réel ; `OcrEngine::MaxImageDimension()` (`Ocr/mod.rs:99`).

**`recognize`** :
```rust
if width > self.max_dim || height > self.max_dim { return Err(EngineError::RenderTooLarge); }   // sinon E_INVALIDARG 0x80070057 (mesuré)
let bgra = rgba_to_bgra(rgba);                         // copie + permutation R/B (fonction pure testée)
let writer = DataWriter::new()?;                       // Storage/Streams/mod.rs:308
writer.WriteBytes(&bgra)?;                             // :352
let buffer = writer.DetachBuffer()?;                   // :446  → IBuffer, sans fichier
let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width as i32, height as i32)?;  // Graphics/Imaging/mod.rs:2295, Bgra8 :768
let result = self.engine.RecognizeAsync(&bitmap)?.join()?;   // Ocr/mod.rs:81 ; windows-future join.rs:12
for each line in result.Lines()? {                     // Ocr/mod.rs:192 ; IVectorView::Size/GetAt : windows-collections bindings.rs:1735/1723
    for each word in line.Words()? {                   // :160
        let r = word.BoundingRect()?;                  // :231 ; Rect { X, Y, Width, Height: f32 } Foundation/mod.rs:2418
        let text = word.Text()?.to_string_lossy();     // :238
    } }
text_angle_deg = result.TextAngle().ok().and_then(|a| a.Value().ok()).map(|v| v as f32)   // :199 ; IReference::Value Foundation/mod.rs:1311
```
Toute `windows::core::Error` est convertie par `fn failed(e) -> EngineError::OcrFailed(format!("0x{:08X} {e}", e.code().0 as u32))` (`error.rs:123` `code()`, `:213` `Display`). On boucle avec `Size()`/`GetAt()` plutôt qu'avec `IntoIterator` (qui fait `First().unwrap()`, `bindings.rs:1803`). `OcrEngine` est créé **une fois par tâche**, pas par page.

### 3.7 Régénération du contenu et instantanés

- Le contenu de la page est régénéré une seule fois à la fin de `ocr_page` (comme tout outil d'édition de `edit.rs`). PDFium réécrit le flux de contenu de **cette page** ; les images restent
  référencées (pas de ré-encodage), ce que le test de rendu identique (8.2, test 1) vérifie octet par octet. Les pages ignorées ne sont pas touchées.
- Annulation Ctrl+Z : un seul instantané pris au début de la tâche par le worker (4) ; `State::push_undo` à la fin, **seulement si au moins une page est `Done`**. Échec : `doc.restore(snapshot)` (même chemin que `State::edit`, `worker.rs:387`).
- Documents chiffrés : `snapshot`/`restore` réutilisent le mot de passe mémorisé (`pages.rs:250-273`) ; rien de spécifique.
- Après OCR, `Response::DocumentChanged` (comme les autres modifications) vide textures, miniatures et résultats de recherche (`app.rs:422-442`) : la recherche suivante voit la nouvelle couche.

### 3.8 Texte invisible et éditeur (`edit.rs`)

Problème : l'outil « Sélectionner un objet » appelle `object_at`, qui prend l'objet **le plus en avant** sous le point ; les mots invisibles (ajoutés en dernier) masqueraient l'image numérisée.
Correctif : `ObjectInfo` gagne `pub invisible: bool` (renseigné dans `describe`, `edit.rs:107`, par `object.as_text_object().is_some_and(|t| !t.is_visible())`) et `object_at` (`edit.rs:182-192`) ajoute `.filter(|o| !o.invisible)`.
`ObjectInfo` n'est construit qu'à cet endroit (vérifié par recherche) ; `page_objects` continue de lister les objets invisibles (indices inchangés). Test : 8.2, test 1.

### 3.9 Registre de commandes

`command.rs` : variante `Ocr` dans `enum Command`, et
`cmd!(Ocr, "convert.ocr", "menu.file.ocr", None, true, Convert)` juste après `Optimize` dans `REGISTRY`. Pas de raccourci (les lettres seules sont prises par les outils).
Les tests existants (`ids_are_unique_and_round_trip`, `registry_keys_exist_in_both_catalogs`) couvrent l'unicité de l'id et la présence de la clé dans les deux catalogues.

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

**Pourquoi ni `EditOp` ni `BatchOp`.** `EditOp` s'exécute d'un bloc sous un instantané (`State::edit`, `worker.rs:372`) : une OCR de 100 pages gèlerait le thread moteur sans progression ni annulation.
`BatchOp` ne modifie pas le document ouvert et n'est pas annulable (`worker.rs:30`). On ajoute donc une tâche à état, avancée par la boucle (modèle du pré-rendu, `worker.rs:646-651`).

### 4.1 Nouveaux types

```rust
pub struct OcrJob { pub pages: Vec<usize>, pub language: String /* étiquette installée, ex. « fr-FR » */, pub skip_pages_with_text: bool }

// Request
OcrLanguages,            // → Response::OcrLanguages
OcrStart(OcrJob),
OcrCancel,

// Response
OcrLanguages(Result<OcrEnvironment, EngineError>),
OcrProgress { done: usize, total: usize, page: usize, words: usize },
OcrFinished(OcrSummary),            // précédé de DocumentChanged si au moins une page a changé
OcrFailed(EngineError),
```
`State` gagne `ocr: Option<OcrRun>` ; `OcrRun { recognizer: WinRecognizer, pending: VecDeque<usize>, total: usize, options: OcrOptions, snapshot: Vec<u8>, summary: OcrSummary }`.

### 4.2 Méthodes de `State`

- `ocr_start(&mut self, job) -> Result<Response, EngineError>` : document ouvert requis ; pages non vides et `< page_count` (sinon `PageOutOfRange`) ; refus si une tâche est déjà active ; `WinRecognizer::new(&job.language)?` (initialise WinRT sur ce thread) ;
  `snapshot = doc.snapshot()?` ; `self.ocr = Some(…)` ; renvoie `OcrProgress { done: 0, … }`.
- `ocr_step(&mut self) -> Vec<Response>` : dépile une page, `doc.ocr_page(page, &run.recognizer, &run.options)` ; `Ok(report)` → `summary.pages.push`, `OcrProgress`, et si plus rien en attente `ocr_finish(false)` ;
  `Err(e)` → `ocr_rollback()` puis `[OcrFailed(e)]`.
- `ocr_finish(&mut self, cancelled: bool) -> Vec<Response>` : `take()` de la tâche ; si ≥ 1 page `Done` → `push_undo(snapshot)`, `invalidate()`, `DocumentChanged(doc_state)` ; puis `OcrFinished(summary)`. Annulation explicite = **conserver** l'existant (une entrée d'annulation).
- `ocr_rollback(&mut self)` : `doc.restore(run.snapshot)` sans entrée d'annulation (échec ou conflit).

### 4.3 Branchement dans `run()`

- Boucle (`worker.rs:641-659`) : dans le bras `Err(TryRecvError::Empty)`, **avant** le pré-rendu :
  `if state.ocr.is_some() { for r in state.ocr_step() { if tx.send(r).is_err() { return; } } ctx.request_repaint(); continue; }`.
  Les requêtes entrantes (rendu, miniature, recherche) passent donc toujours avant la page suivante : la latence d'une action d'interface = la durée d'**une** page (mesuré : reconnaissance 56–255 ms ; rendu et écriture non mesurés).
- Traitement des requêtes : `OcrLanguages` → `Response::OcrLanguages(ocr::probe())` ; `OcrStart` → `ocr_start` (erreur → `OcrFailed`) ;
  `OcrCancel` → `ocr_finish(true)` envoyé tel quel puis `continue` (ces bras envoient plusieurs réponses).
- **Conflits** : avant le `match request`, si `state.ocr.is_some()` et que la requête est `Open | Close | Suspend | Resume* | Edit | Undo | Redo | Save | Batch`, appeler `ocr_rollback()` et envoyer `OcrFailed(EngineError::Other("ocr interrupted"))`.
  Les requêtes de lecture (`Render`, `Thumbnail`, `Search`, `TextInRect`, `Annotations`, `Properties`, `Bookmarks`, `ObjectAt`, `ObjectInfo`) continuent normalement. La boîte modale rend ces conflits quasi impossibles ; la règle protège l'intégrité de la pile d'annulation.
- `State` initialisé avec `ocr: None` (`worker.rs:631-637`).

## 5. Interface (`apps/sunipdf-desktop`)

### 5.1 Fichiers

| Fichier | Changement |
|---|---|
| `src/ocr.rs` (nouveau) | `OcrState`, dialogue, gestion des réponses ; `mod ocr;` dans `main.rs` |
| `src/app.rs` | champ `pub ocr: crate::ocr::OcrState` ; arm `Command::Ocr => self.open_ocr_dialog()` dans `execute` (`app.rs:955` voisinage) ; `handle_input` : ajouter `|| self.ocr.dialog_open` à la liste de retour anticipé (`app.rs:591-596`) ; `handle_responses` : 4 nouveaux bras ; `error_key` : 3 nouveaux bras ; `ui()` : `self.ocr_dialog(&ctx);` après `self.stamp_dialog(&ctx);` (`app.rs:1877`) ; test `ui_keys_used_by_app_exist` : ajouter `include_str!("ocr.rs")` ; liste du test d'erreurs étendue |
| `src/shell.rs` | `Command::Ocr` dans `ToolTab::All` et `ToolTab::Convert` (après `Optimize`, `shell.rs:71` et `:101`) ; `command_icon` : `Command::Ocr => (glyph::SCAN, "OCR")` (`shell.rs:290`) ; la couleur vient de `Menu::Convert` → `CAT_CONVERT` (aucun changement dans `command_color`) |
| `src/icons.rs` | `pub const SCAN: &str = "\u{E8FE}";` — « Scan » dans Segoe Fluent Icons (learn.microsoft.com, table des glyphes) ; **présent dans `SegoeIcons.ttf` et `segmdl2.ttf` de ce poste** (mesuré via `GlyphTypeface`) |
| `src/worker.rs` | section 4 |
| `assets/i18n/*.json` | section 7 |

### 5.2 État et interaction

```rust
#[derive(Default)]
pub struct OcrState {
    pub dialog_open: bool,
    pub probe: Probe,                       // Unknown | Checking | Ready(OcrEnvironment) | Unavailable
    pub form: OcrForm,                      // lang: OcrLang (Fr | En), scope: OcrScope (All | Current | Selected), skip_text: bool (défaut true)
    pub run: Option<OcrRunUi>,              // { done, total, words, cancelling }
}
```
- `open_ocr_dialog` : `dialog_open = true`, `probe = Checking`, `Request::OcrLanguages`. Langue par défaut : celle de l'interface si son pack est installé, sinon l'autre si installée, sinon celle de l'interface (avertissement visible). Portée « Sélection » proposée seulement si la vue Organiser est active avec une sélection (`target_pages()`, `organize.rs:25`).
- `ocr_dialog(ctx)` passe **obligatoirement** par `dialog::show` (règle D020) : `Spec::new("ocr", tr(lang,"ocr.title"), dark, has_icons).subtitle(tr(lang,"ocr.subtitle")).icon(glyph::SCAN, "OCR").width(520.0)`.
  - Corps, mode formulaire : liste « Langue du texte » en deux `radio_value` (Français / Anglais) avec pastille « installée (fr-FR) » / « non installée » ; note « une seule langue par passage » ; portée (3 `radio_value`) ; case `skip_text` avec son aide ;
    si la langue choisie manque : encart (`Frame` teinté `theme::ERROR`) avec `ocr.missing.title/how/restart`, boutons `dialog::secondary` « Ouvrir les paramètres de langue » (lance `explorer.exe ms-settings:regionlanguage` via `std::process::Command`, **à vérifier à la recette**) et « Vérifier à nouveau » (renvoie `OcrLanguages`) ;
    `Probe::Unavailable` → texte `ocr.unavailable` seul ; `Probe::Checking` → spinner + `ocr.checking`.
  - Corps, mode exécution (`run.is_some()`) : `egui::ProgressBar::new(done as f32 / total as f32).show_percentage()` (`egui-0.36.2/src/widgets/progress_bar.rs:27,69`) et `ocr.progress` ; texte `ocr.cancelling` quand l'annulation est demandée.
  - Pied (premier bouton ajouté = le plus à droite) : formulaire → `ui.add_enabled_ui(langue_installée, |ui| dialog::primary(ui, tr(lang,"ocr.start")))` (`ui.rs:1620`) puis `dialog::secondary(ui, tr(lang,"dialog.password.cancel"))` (clé existante, comme `tools.rs:283`) ;
    exécution → `dialog::secondary(ui, tr(lang,"ocr.cancel"))` (Échap équivalent) → `Request::OcrCancel`, `run.cancelling = true`.
  - Lancer : construit `OcrJob { pages, language: étiquette installée, skip_pages_with_text }`, `run = Some(…)`, `worker.send(Request::OcrStart(job))`.
- Réponses : `OcrLanguages(Ok(env))` → `probe = Ready(env)` ; `Err(OcrUnavailable)` → `Unavailable` ; autre erreur → bannière d'erreur. `OcrProgress` → met à jour `run`. `OcrFinished(summary)` → ferme le dialogue, `status_msg` =
  fonction pure `ocr_message(lang, &summary)` : `ocr.nothing_to_do` si rien traité et ≥ 1 page ignorée ; `ocr.cancelled` si annulé ; sinon `ocr.done` suivi (séparé par une espace) de `ocr.done_extra` si des pages ont été ignorées ou sans texte.
  `OcrFailed(e)` → ferme le dialogue, `self.error = Some(error_key(&e))`.
- **Onglets** : la boîte est modale (barre d'onglets inaccessible) ; la tâche appartient au document actif du moteur. Un `Suspend` imprévu annule et restaure (4.3), l'état `OcrState` n'est pas par onglet.
- **Annulation/rétablissement** : une entrée ; `DocumentChanged` met `can_undo`.
- **Accessibilité** : `a11y.ocr_progress` posé sur la barre via `.text(...)`/`widget_info` ; le dialogue prend le focus clavier (Enter = action principale si active, Échap = annuler).

### 5.3 Accueil

`home.card.convert.desc` (déjà existante) devient « Images → PDF · PDF → PNG · OCR · Optimiser » (les deux catalogues).

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

```rust
/// Add an invisible OCR text layer (Windows.Media.Ocr, offline) so scanned pages become searchable.
Ocr {
    file: PathBuf,
    /// fr | en | a BCP-47 tag installed in Windows (see ocr-languages).
    #[arg(long, default_value = "en")] lang: String,
    #[arg(long, default_value = "all")] pages: String,          // même syntaxe que les autres commandes (parse_pages)
    /// OCR pages that already contain text (replaces previous invisible text).
    #[arg(long)] force: bool,
    #[arg(long, default_value_t = 300.0)] dpi: f32,
    /// One JSON line per page on stderr.
    #[arg(long)] progress: bool,
    #[arg(long)] out: PathBuf,
    #[arg(long)] password: Option<String>,
},
/// List the OCR recognizer languages installed in Windows.
OcrLanguages,
```
`run()` : `probe()` → `resolve_language(&env, &lang)` → `WinRecognizer::new` → `doc.ocr_pages(&selection, &rec, &OcrOptions { dpi, skip_pages_with_text: !force }, &mut cb)` → `doc.save_as(&out)` (écriture atomique, **rien n'est écrit si l'OCR échoue**).
`main()` : `EngineError::OcrUnavailable | EngineError::OcrLanguageMissing(_) => ExitCode::from(6)`.

Sortie `ocr` (stdout) :
```json
{"out":"scan-ocr.pdf","engine":"windows.media.ocr","language_requested":"fr","language_used":"fr-FR",
 "pages_total":2,"pages_processed":2,"pages_skipped_text":0,"pages_without_text":0,"words":118,"seconds":1.42,
 "pages":[{"page":1,"outcome":"done","words":61,"lines":5,"dpi":300.0,"text_angle_deg":0.0}]}
```
`outcome` ∈ `done | skipped_text | no_text_found`. Sortie `ocr-languages` :
`{"engine":"windows.media.ocr","max_image_dimension":10000,"languages":[{"tag":"en-GB","display_name":"English (United Kingdom)"}]}` (mesuré côté PowerShell).
Erreurs : JSON sur stderr `{ "error": "ocr_language_missing", "message": … }`, code 6.
Le registre exposé par `sunipdf-cli commands` contient automatiquement `convert.ocr`.

## 7. i18n — clés à ajouter dans `fr-FR.json` **et** `en-US.json`

Espaces utilisés : `menu.file.*` (convention des commandes Convertir existantes), `ocr.*` (réservé dans `docs/I18N.md`), `error.*`, `a11y.*`. Les paramètres `{…}` sont identiques dans les deux langues.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.file.ocr` | Reconnaissance de texte (OCR)… | Text Recognition (OCR)… |
| `ocr.title` | Reconnaissance de texte (OCR) | Text Recognition (OCR) |
| `ocr.subtitle` | Ajoute une couche de texte invisible aux pages numérisées : elles deviennent recherchables et leur texte sélectionnable, sans changer leur aspect. | Adds an invisible text layer to scanned pages so they become searchable and their text selectable, without changing how they look. |
| `ocr.checking` | Recherche des langues OCR installées dans Windows… | Checking the OCR languages installed in Windows… |
| `ocr.unavailable` | La reconnaissance de texte de Windows n'est pas disponible sur ce système. | Windows text recognition is not available on this system. |
| `ocr.language` | Langue du texte | Text language |
| `ocr.lang.fr` | Français | French |
| `ocr.lang.en` | Anglais | English |
| `ocr.lang.installed` | installée ({tag}) | installed ({tag}) |
| `ocr.lang.missing` | non installée | not installed |
| `ocr.lang.note` | Une seule langue par passage : choisissez celle du texte principal. | One language per run: pick the main language of the text. |
| `ocr.missing.title` | Le pack OCR « {language} » n'est pas installé sur ce Windows. | The “{language}” OCR pack is not installed on this Windows. |
| `ocr.missing.how` | Ouvrez Paramètres › Heure et langue › Langue et région, ajoutez la langue puis activez « Reconnaissance optique de caractères ». En administrateur : Add-WindowsCapability -Online -Name Language.OCR~~~{tag}~0.0.1.0 | Open Settings › Time & language › Language & region, add the language, then enable “Optical character recognition”. As administrator: Add-WindowsCapability -Online -Name Language.OCR~~~{tag}~0.0.1.0 |
| `ocr.missing.restart` | Si le pack n'apparaît pas après l'installation, relancez SuniPDF. | If the pack does not appear after installation, restart SuniPDF. |
| `ocr.missing.open_settings` | Ouvrir les paramètres de langue | Open language settings |
| `ocr.missing.recheck` | Vérifier à nouveau | Check again |
| `ocr.scope` | Pages à traiter | Pages to process |
| `ocr.scope.all` | Toutes les pages ({count}) | All pages ({count}) |
| `ocr.scope.current` | Page courante ({page}) | Current page ({page}) |
| `ocr.scope.selected` | Pages sélectionnées ({count}) | Selected pages ({count}) |
| `ocr.skip_text` | Ignorer les pages qui contiennent déjà du texte | Skip pages that already contain text |
| `ocr.skip_text.hint` | Décochée : le texte invisible d'un OCR précédent est remplacé ; le texte visible est conservé. | Unchecked: invisible text from a previous OCR is replaced; visible text is kept. |
| `ocr.start` | Lancer l'OCR | Run OCR |
| `ocr.cancel` | Annuler l'OCR | Cancel OCR |
| `ocr.cancelling` | Annulation en cours… | Cancelling… |
| `ocr.progress` | Page {current} sur {total} · {words} mot(s) reconnu(s) | Page {current} of {total} · {words} word(s) recognized |
| `ocr.done` | OCR terminé : {pages} page(s) traitée(s), {words} mot(s) reconnu(s). | OCR complete: {pages} page(s) processed, {words} word(s) recognized. |
| `ocr.done_extra` | {skipped} ignorée(s) (texte déjà présent), {empty} sans texte reconnu. | {skipped} skipped (text already present), {empty} with no text found. |
| `ocr.nothing_to_do` | Aucune page à traiter : elles contiennent déjà du texte. | Nothing to do: the pages already contain text. |
| `ocr.cancelled` | OCR annulé : {pages} page(s) conservée(s) (Ctrl+Z pour annuler). | OCR cancelled: {pages} page(s) kept (press Ctrl+Z to undo). |
| `error.ocr_unavailable` | La reconnaissance de texte de Windows n'est pas disponible sur ce système. | Windows text recognition is not available on this system. |
| `error.ocr_language_missing` | Le pack OCR de cette langue n'est pas installé dans Windows (Paramètres › Heure et langue › Langue et région). | The OCR pack for this language is not installed in Windows (Settings › Time & language › Language & region). |
| `error.ocr_failed` | La reconnaissance de texte a échoué ; le document n'a pas été modifié. | Text recognition failed; the document was not modified. |
| `a11y.ocr_progress` | Progression de la reconnaissance de texte : {percent} % | Text recognition progress: {percent}% |

Soit **34 nouvelles clés** (1 `menu`, 29 `ocr`, 3 `error`, 1 `a11y` ; à recompter au moment de l'ajout) et **1 clé modifiée** : `home.card.convert.desc` →
fr « Images → PDF · PDF → PNG · OCR · Optimiser », en « Images → PDF · PDF → PNG · OCR · Optimize ». Le test de parité (`crates/sunipdf-i18n`) et `scripts\check-i18n.ps1` valident les catalogues ;
l'étiquette « Paramètres › … » reprend le libellé de ce poste, à confirmer en français à la recette. Aucune chaîne littérale dans `apps/` : les noms de langue viennent des clés `ocr.lang.*`, et
`display_name` (fourni par Windows) n'est affiché qu'en info-bulle.

## 8. Tests

### 8.1 Fixtures (`scripts/make_fixtures.py`, sans dépendance, régénération par `python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`)

1. `ocr-source.pdf` — 2 pages Lettre 612×792, Helvetica 14 pt avec `/Encoding /WinAnsiEncoding` (indispensable pour les accents), lignes tous les 24 pt à partir de y = 720 :
   page 1 (anglais) « Invoice number 2024-0173 », « The quick brown fox jumps over the lazy dog. », « Total amount due: 1,250.50 EUR before March 31. », « Please send the payment to the account below. », « Thank you for your business and your trust. » ;
   page 2 (français, octets cp1252 `\xe9 \xe8 \xe0 \x80`) « Facture numéro 2024-0173 », « L'école est fermée dès l'été, où allez-vous ? », « Les élèves ont déjà lu le texte français. », « Je m'en vais à la plage et j'ai payé 12,50 €. », « Merci de votre confiance et à bientôt. ».
   **La vérité terrain n'est jamais recopiée dans les tests** : elle est lue par `source.page_text(i)` (une source, aucune divergence possible).
2. `ocr-offset.pdf` — 1 page à `/MediaBox [50 100 662 892]` avec une ligne de texte (page qui « a du texte » → tests avec `skip_pages_with_text: false`).
Le document « numérisé » est produit **au moment du test** (aucun binaire versionné) : `source.export_images(dir, 300.0)` (`convert.rs:204`) puis `engine.images_to_pdf(&pngs, &out, 300.0)` (`convert.rs:162`) → pages 612×792 pt sans texte.

### 8.2 Tests automatiques

**Unitaires** (`ocr.rs`/`ocr/winrt.rs`, sans PDFium ni Windows) : `display_to_user` (table 3.5, 4 rotations + coins) ; `sanitize_word` (« école » intact, « ﬁn » → « fin », « 日本 » → ignoré, contrôle supprimé, `€` conservé) ;
`render_scale` (A4 et A0) ; `rgba_to_bgra` ; `resolve_language` (installé `fr-CA` pour « fr », absent → « fr-FR ») ; `OcrSummary::{processed, skipped_text, empty, words}`.

**Intégration** (`crates/sunipdf-core/tests/engine.rs`, helpers `engine()` et `fixture()` existants) avec un `FakeRecognizer` déterministe qui renvoie des mots à des **fractions** de l'image (indépendant du dpi) :

```rust
struct Fake { words: Vec<(&'static str, f32, f32, f32, f32)>, fail: bool }   // texte, x, y, w, h en fractions
impl Recognizer for Fake { /* language_tag "xx-fake", max_image_dimension 10_000, recognize → mots × (w,h) */ }
```
1. `ocr_text_layer_is_invisible_searchable_and_persistent` — document numérisé (page 1 de `ocr-source.pdf`), précondition `page_text(0).trim().is_empty()` ; rendu 1,0 avant ; `ocr_pages` avec mots factices « Facture » (0.12,0.10,0.20,0.025), « école » (0.36,0.10,0.12,0.025), « ﬁnale » ;
   attendus : `Done`, 3 mots ; `page_text` contient « Facture », « école », « finale » ; `search_page(0,"Facture")` donne un rectangle à ±3 pt de (73,44 ; 693,0 ; 195,84 ; 712,8) (fractions × 612×792, origine bas-gauche) ;
   **rendu après == rendu avant, octet pour octet** (le texte est invisible et l'image intacte) ; `object_at` au centre renvoie `ObjectKind::Image` (pas du texte) ; 3 objets texte avec `invisible == true` ;
   `save_as` + réouverture : `page_text`, recherche, rendu identique et `invisible == true` toujours vrai (prouve que `3 Tr` survit à l'enregistrement par pdfium-render) ;
   relance avec options par défaut → `SkippedHasText`, nombre d'objets inchangé ; relance avec `skip_pages_with_text: false` → toujours 3 objets texte (remplacés, pas dupliqués) et un seul résultat pour « Facture ».
2. `ocr_rotated_pages_place_text_in_user_space` — `set_page_rotation(0, R90)` puis `R270` sur un document numérisé (taille affichée 792×612) ; mêmes mots factices ; les rectangles trouvés sont ceux calculés **à la main dans le test** (R90 : x = v, y = u → « Facture » x 61,2..76,5 ; y 95,0..253,4, tolérance 3 pt ; R270 : x = H − v, y = W − u) et leur hauteur dépasse trois fois leur largeur (texte vertical).
3. `ocr_skips_pages_with_text_and_respects_origin` — `two-pages.pdf` page 0 : `SkippedHasText`, document inchangé ; avec `skip_pages_with_text: false` : texte visible conservé et mots ajoutés ; `ocr-offset.pdf` : rectangle attendu = fractions × taille + (50, 100) à ±3 pt.
4. `ocr_failure_and_cancel_leave_document_consistent` — reconnaisseur en échec : `Err(OcrFailed)` et nombre d'objets de la page inchangé (`restore`) ; annulation (`ControlFlow::Break` après la page 1 d'un document numérisé de 2 pages) : `cancelled == true`, page 1 a sa couche, `page_text(1)` vide ; `ocr_page` hors limites → `PageOutOfRange` ; `dpi` invalide (NaN, 0) → erreur.
5. `ocr_windows_english_end_to_end` — **réel, conditionnel** : `WinRecognizer::new("en-US")` ; `Err(OcrUnavailable | OcrLanguageMissing(_))` → `eprintln!` + `return` (le test passe en se sautant, **à signaler dans le rapport, jamais à compter comme une vérification**). Sinon : document numérisé de la page 1, `ocr_pages`, vérité = `source.page_text(0)`, texte OCR = `doc.page_text(0)`,
   **CER** = distance de Levenshtein / longueur de la vérité après normalisation des espaces (fonction de 12 lignes dans le test), imprimé avec `eprintln!` (« CER en = 0.0xx, N car., T ms/page »), et assertion tolérante : ≥ 80 % des mots distincts de ≥ 4 lettres de la vérité sont trouvés par `search_page`. Seuil à resserrer après la première exécution réelle.
6. `ocr_windows_french_end_to_end` — idem avec la page 2 et `"fr-FR"`, plus `search_page("école")`, `("fermée")`, `("français")` ; **sur ce poste ce test se saute** (pack absent) ; seuil provisoire 70 %, à calibrer sur un poste équipé.
7. `ocr_windows_language_errors` — `WinRecognizer::new("xx-XX")` et `new("")` renvoient une erreur de code `ocr_language_missing` ou `ocr_unavailable` ; `probe()` ne panique pas.

**Application** : `every_error_key_exists_in_both_catalogs` étendu (3 variantes) ; `ui_keys_used_by_app_exist` inclut `ocr.rs` ; nouveau test pur `ocr_message_variants` (trois cas du message de fin).
**Commandes** (non exécutées ici) : voir 8.3.

### 8.3 Commandes de vérification (à lancer par l'agent principal, `-j 2`)

```powershell
. .\scripts\dev-env.ps1
python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures
cargo test -p sunipdf-core -j 2 --lib ocr
cargo test -p sunipdf-core -j 2 --test engine ocr -- --nocapture      # imprime le CER et les ms/page des tests réels
cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo fmt --all -- --check
.\scripts\check-i18n.ps1
.\target\debug\sunipdf-cli.exe ocr-languages
```
Pour le CER sur plusieurs échantillons (décision Tesseract ultérieure) : produire des scans avec `sunipdf-cli export-images --dpi 300` + `images2pdf`, lancer `sunipdf-cli ocr`, comparer `sunipdf-cli text` à la vérité terrain ; consigner dans `docs/OCR_EVALUATION.md` (corpus, commande, version de Windows, packs installés).

### 8.4 Scénario manuel de captures (`scripts/dev-screenshot.ps1 -Pdf -Commands -Drag -Keys -KeysAfter -Delay`)

```powershell
# 1. PDF « numérisé » de 2 pages puis dupliqué pour voir une progression (CLI existante)
.\target\debug\sunipdf-cli.exe export-images crates\sunipdf-core\tests\fixtures\ocr-source.pdf --out-dir $env:TEMP\ocr --dpi 300
.\target\debug\sunipdf-cli.exe images2pdf $env:TEMP\ocr\ocr-source-1.png $env:TEMP\ocr\ocr-source-2.png --out $env:TEMP\ocr\scan.pdf --dpi 300
.\target\debug\sunipdf-cli.exe merge $env:TEMP\ocr\scan.pdf $env:TEMP\ocr\scan.pdf $env:TEMP\ocr\scan.pdf --out $env:TEMP\ocr\scan6.pdf

# 2. Boîte de dialogue (état réel de ce poste : français non installé → encart d'aide)
.\scripts\dev-screenshot.ps1 -Out ocr-dialog.png -Pdf "$env:TEMP\ocr\scan6.pdf" -Commands 'convert.ocr'

# 3. Lancement : l'anglais est choisi par défaut ici ; Entrée = action principale ; capture après traitement
.\scripts\dev-screenshot.ps1 -Out ocr-done.png -Pdf "$env:TEMP\ocr\scan6.pdf" -Commands 'convert.ocr' -KeysAfter '{ENTER}' -Delay 6

# 4. Preuve de recherche sur le résultat de la CLI (surbrillance alignée sur le texte numérisé)
.\target\debug\sunipdf-cli.exe ocr $env:TEMP\ocr\scan.pdf --lang en --pages 1 --out $env:TEMP\ocr\scan-ocr.pdf
.\scripts\dev-screenshot.ps1 -Out ocr-find.png -Pdf "$env:TEMP\ocr\scan-ocr.pdf" -Keys '^fquick{ENTER}'

# 5. Sélection de texte sur la page numérisée puis copie (coordonnées à calibrer par une première capture)
.\scripts\dev-screenshot.ps1 -Out ocr-select.png -Pdf "$env:TEMP\ocr\scan-ocr.pdf" -Drag '430,180,760,180' -KeysAfter '^c'

# 6. Même vérification sur une page pivotée (géométrie 90°) : sunipdf-cli rotate puis ocr puis -Keys '^f...'
.\target\debug\sunipdf-cli.exe rotate $env:TEMP\ocr\scan.pdf --pages 1 --degrees 90 --out $env:TEMP\ocr\scan-r90.pdf
.\target\debug\sunipdf-cli.exe ocr $env:TEMP\ocr\scan-r90.pdf --lang en --pages 1 --out $env:TEMP\ocr\scan-r90-ocr.pdf
.\scripts\dev-screenshot.ps1 -Out ocr-r90.png -Pdf "$env:TEMP\ocr\scan-r90-ocr.pdf" -Keys '^fquick{ENTER}'
```
À contrôler à l'œil : barre de progression et libellés de la capture 3 (si le traitement est trop rapide, la capture montre l'état final : utiliser `scan6.pdf` multiplié), alignement de la surbrillance (4, 6), message d'état, Ctrl+Z (annule toute la couche),
thème sombre. Recette française : installer le pack (Paramètres) sur un poste de test puis relancer 2 et 3 avec `--lang fr`.

## 9. Nouvelles dépendances

| Crate | Version | Licence | Pourquoi |
|---|---|---|---|
| `windows` (dépendance **directe** de `sunipdf-core`) | 0.62 — **0.62.2 déjà verrouillée** (tirée par `accesskit_windows` 0.32.1) | MIT OR Apache-2.0 | `Windows.Media.Ocr`, `SoftwareBitmap`, `DataWriter`, `RoInitialize` |

```toml
# crates/sunipdf-core/Cargo.toml
windows = { version = "0.62", features = ["Foundation", "Foundation_Collections", "Globalization", "Graphics_Imaging", "Media_Ocr", "Storage_Streams", "Win32_System_WinRT"] }
```
(`Foundation_Collections` par prudence ; les types `IVectorView`/`IAsyncOperation` viennent de `windows-collections` 0.3.2 / `windows-future` 0.3.2, déjà verrouillés.) Aucun paquet nouveau dans `Cargo.lock`, seulement une arête de dépendance ;
pas de DLL ni de modèle à livrer (`packaging/` inchangé). `windows` n'a aucun appel réseau. Première compilation : lancer seule `cargo check -p sunipdf-core -j 2` (14 Go) car les features `Graphics_Imaging` et `Storage_Streams` ajoutent du code généré.
Le projet reste Windows-only (CI `windows-latest`) : pas de `cfg` de repli pour les autres systèmes.

## 10. Risques, pièges, ordre d'implémentation, estimation

### 10.1 Risques et pièges

- **R1 Pack français absent ici** : le chemin français réel ne sera vérifié qu'après installation ; d'ici là seule la logique est testée (moteur factice + accents WinAnsi). Ne pas cocher « OCR français » dans `PROJECT_STATUS.md` avant la recette.
- **R2 Précision non mesurée** : 1 image synthétique seulement (2.3). Ne présenter aucun CER « corpus » ; seuils des tests réels provisoires.
- **R3 `3 Tr` via pdfium-render** : validé au niveau C sur `pdfium.dll` 7881 (2.3) mais l'appel Rust `set_render_mode` n'a pas été exécuté ; le test 1 (réouverture + rendu identique + `invisible`) est le garde-fou. Si le texte devenait visible, il apparaîtrait comme un écart de rendu.
- **R4 Régénération du flux de contenu** d'une page (comme toute édition) : images conservées par référence ; le test de rendu octet-pour-octet le prouve sur la fixture. Pages à contenu vectoriel complexe : même risque que les outils d'édition existants.
- **R5 Boîtes** : PDFium et Windows donnent des boîtes d'**encre** ; les mots d'un seul signe de ponctuation donnent des facteurs d'échelle extrêmes (ignorés sous 0,1 pt). Surbrillance non rectiligne d'un mot à l'autre (même comportement qu'un PDF natif).
- **R6 Threading COM** : `RoInitialize(MTA)` sur le thread moteur ; tout futur code qui y initialiserait un STA recevrait `RPC_E_CHANGED_MODE`. Ne jamais appeler l'OCR depuis le fil egui.
- **R7 Blocage** : `join()` n'a pas de délai ; un appel WinRT qui ne rend jamais gèlerait **tout** le moteur (rendu compris). Probabilité faible, impact fort → durcissement 10.3.
- **R8 Mémoire** : ≈ 140 Mo en crête pour une page A4 à 300 ppp, ≈ 280 Mo pour A3 (estimation à partir de 4 octets/pixel × 3–4 copies) ; plus l'instantané du document (taille du PDF) conservé pour Ctrl+Z.
- **R9 Séparateurs** : « 1 ,250.50 » scindé (mesuré) → la recherche « 1,250.50 » ne trouve rien ; limite documentée.
- **R10 Éditions de Windows** : comportement sans composant OCR non vérifié (2.5) ; `ms-settings:` lancé via `explorer.exe` non vérifié ; libellé français de l'option Paramètres non vérifié.
- **R11 Confidentialité** : ne pas écrire « hors ligne » dans le guide avant une capture réseau (Moniteur de ressources/`netstat`) pendant un OCR ; l'installation du pack passe par Windows Update (hors SuniPDF).
- **R12 Conflits de fusion avec les autres tranches** (`docs/plans/*.md`) : `command.rs` (enum + REGISTRY), `shell.rs` (listes d'onglets, `command_icon`), `app.rs` (`execute`, `error_key`, `handle_responses`, `ui()`), `worker.rs` (Request/Response), `edit.rs` (`ObjectInfo`), `error.rs`, `home.card.convert.desc`. Ajouts purement additifs ; fusionner en rebasant, recompter les clés i18n.
- **R13 Compilation** : `windows` + features supplémentaires ; surveiller la mémoire (`-j 2`). Si le build `Manual` pose problème, repli : stratégie par défaut (plus lent mais sûr).
- **R14 Tests réels non déterministes** : sortie dépendante de la version de Windows et du pack ; assertions en proportions, pas en égalité de texte.
- **R15 Fichier plus gros** (≈ +30 Ko pour 500 mots, non mesuré) ; mesurer à l'étape 5 avec `optimize`.

### 10.2 Ordre d'implémentation (chaque étape compile et passe ses tests)

1. **Erreurs + types purs** : `error.rs` (3 variantes + `error_key` + test de clés + catalogues `error.ocr_*`), `ocr.rs` sans `ocr_page` (types, trait, `display_to_user`, `sanitize_word`, `render_scale`, `OcrSummary`) + tests unitaires. *Sortie : `cargo test -p sunipdf-core --lib ocr` vert.*
2. **Écriture de la couche** : `ocr_page`, `page_has_text`, `ocr_pages`, `ObjectInfo.invisible` + `object_at`, fixtures 8.1, `FakeRecognizer` et tests 1 à 4. *Sortie : tests d'intégration verts ; c'est ici que se confirment `set_render_mode`, `bounds()` avant insertion, `Manual`, la rotation et l'origine.*
3. **Windows** : dépendance `windows`, `ocr/winrt.rs` (`probe`, `resolve_language`, `WinRecognizer`), tests 5 à 7. *Sortie : `ocr-languages` fonctionne ; test anglais réel avec CER imprimé ; français sauté ici.*
4. **CLI** : `ocr`, `ocr-languages`, code 6. *Sortie : scénario 8.4 (1, 4, 6) sur ce poste.*
5. **Worker** : `OcrJob`, requêtes/réponses, `OcrRun`, branchement de la boucle, règle de conflits. *Sortie : compile ; test manuel sans interface via `SUNIPDF_STARTUP_COMMANDS`.*
6. **Interface** : `Command::Ocr` (registre, icône `SCAN`, listes d'outils), `ocr.rs`, `app.rs`, catalogues, `home.card.convert.desc`, test de clés. *Sortie : captures 8.4 (2, 3, 5), `check-i18n.ps1`, clippy, fmt.*
7. **Documentation et recette** (section 11), installation du pack français sur un poste de test et exécution du test 6 ; consigner les mesures réelles.

### 10.3 Durcissements optionnels (après recette)

- **Délai par page** : remplacer `join()` par `RecognizeAsync(..)?.SetCompleted(&AsyncOperationCompletedHandler::new(move |_, _| { let _ = tx.send(()); Ok(()) }))` (`windows-future bindings.rs:1670`, constructeur `:483`), `rx.recv_timeout(60 s)`, puis `GetResults()` (`:1694`) ; au dépassement `Cancel()` (`:1738`) et `OcrFailed("timeout")`.
- **`Gray8`** à la place de `Bgra8` (quatre fois moins de mémoire, plus rapide : 56 ms contre 91 ms mesurés), avec repli `Bgra8` si le moteur refuse.
- **Case « Créer un PDF à partir d'images puis OCR »** dans `tool_create_from_images` (suite directe de « Scan & OCR »).
- **Backend Tesseract** (ADR ultérieur) derrière le trait `Recognizer`, `fra+eng` en un passage ; évaluation `tesseract-static`, builds vcpkg, poids du MSI (≈ 5,2 Mo de `tessdata_fast`).

### 10.4 Estimation : **L**

Environ 900 à 1 200 lignes de Rust (moteur 450, worker 150, interface 250, CLI 80) + 350 lignes de tests + 2 fixtures + 35 lignes de catalogues par langue + documentation. Les étapes 1 à 3 portent le risque technique ; 4 à 6 sont mécaniques (motifs existants). Hors recette française (dépend d'une action utilisateur sur le poste).

## 11. Documentation à mettre à jour

- `docs/DECISIONS.md` : **D021** — OCR par `Windows.Media.Ocr` via la crate `windows` (MIT OR Apache-2.0), module `sunipdf-core::ocr`, pas de crate `sunipdf-ocr` ; Tesseract reporté (poids ≈ 5,2 Mo `tessdata_fast`, vcpkg/clang absents, aucune mesure CER comparée) ; couche de texte Helvetica invisible par mot ; licences : aucune ressource tierce livrée. **D022** (optionnel) — tâches longues du worker avancées page par page par la boucle moteur.
- `docs/PROJECT_STATUS.md` : ligne SUNI-040 (OCR) **seulement avec les résultats réels** des commandes 8.3 ; retirer « OCR » de « Non réalisé » ; mentionner que le français n'est vérifié que si le test 6 a tourné sur un poste équipé.
- `docs/NEXT_TASKS.md` : entrée SUNI-040 terminée ; suites (Tesseract, CER sur corpus dans `docs/OCR_EVALUATION.md`, création image → PDF + OCR, délai par page).
- `docs/FEATURE_MATRIX.md` (section D, ligne OCR) : « **Fait** (moteur Windows, fr/en selon packs installés) ; Tesseract planifié ».
- `docs/ROADMAP.md` (phase 4) et `docs/TECH_STACK.md` (ligne OCR) : refléter D021.
- `docs/ARCHITECTURE.md` : ligne `sunipdf-ocr` → « non créé : module `sunipdf-core::ocr` (D021) » ; ajouter le flux `OcrStart/OcrProgress/OcrFinished` au schéma du worker.
- `docs/KNOWN_ISSUES.md` : nouvelles entrées (pack de langue Windows requis et français absent sur le poste de dev ; une langue par passage ; pas de redressement ; nombres scindés ; texte Helvetica étiré ; non PDF/A ni balisé ; taille du fichier ; pas de délai par page ; éditions de Windows non vérifiées).
- `docs/THIRD_PARTY_NOTICES.md` : « OCR : composant Windows, aucune ressource tierce livrée ; Tesseract/tessdata non livrés (Apache-2.0) » ; la crate `windows` figure déjà dans la génération `cargo about`.
- `docs/I18N.md` : `ocr.*` n'est plus seulement « réservé ».
- `docs/USER_GUIDE.fr.md` / `.en.md` : section OCR (usage, installation du pack de langue, limites) ; retirer « pas d'OCR » de la ligne de limites (`USER_GUIDE.fr.md:124`, `.en.md:118`).
- `CHANGELOG.md` : entrée de version (numéro à décider par l'agent principal).
- `docs/OCR_EVALUATION.md` (créé en suivi, périmètre de l'agent OCR) : protocole CER 8.3 et résultats.

## Annexe A — Citations des sources locales

Préfixes : `PR` = `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src/pdf/document/` ; `WIN` = `…/windows-0.62.2/src/Windows/` ; `WC` = `…/windows-core-0.62.2/src/` ;
`WF` = `…/windows-future-0.3.2/src/` ; `WCO` = `…/windows-collections-0.3.2/src/` ; `WS` = `…/windows-strings-0.5.1/src/` ; `WR` = `…/windows-result-0.4.1/src/`.

| Appel | Source |
|---|---|
| `PdfPageTextObject::new(document, text, font, size)` | `PR page/object/text.rs:197` |
| `PdfPageTextRenderMode::Invisible`, `set_render_mode`, `is_visible` | `PR page/object/text.rs:73`, `:424`, `:253` |
| `FPDFText_SetText_str` (UTF-16) | `pdfium-render-0.9.4/src/bindings.rs:7607` |
| `translate/scale/rotate_clockwise_degrees` sur `PdfPageTextObject` | `PR page/object/text.rs:517` (macro `src/pdf/transform.rs:8`) |
| `bounds()` (impl générique `PdfPageObjectCommon`) | `PR page/object.rs:862-872` (trait `:679`) |
| `as_text_object()` | `PR page/object.rs:433` |
| `add_text_object`, `remove_object_at_index` | `PR page/objects/common.rs:127`, `:368` |
| `set_content_regeneration_strategy`, `regenerate_content`, `Drop` (pas de régénération en `Manual`) | `PR page.rs:909`, `:942`, `:998-1006` |
| `PdfFontToken` (`Copy`), `helvetica()` | `PR fonts.rs:83-86`, `:194` |
| `boundaries()`, `crop()`, `media()`, `PdfPageBoundaryBox.bounds` | `PR page.rs:401`, `page/boundaries.rs:220`, `:113`, `:27-30` |
| `OcrEngine::{RecognizeAsync, RecognizerLanguage, MaxImageDimension, AvailableRecognizerLanguages, TryCreateFromLanguage}` | `WIN Media/Ocr/mod.rs:81, 92, 99, 106, 123` |
| `OcrResult::{Lines, TextAngle}`, `OcrLine::{Words, Text}`, `OcrWord::{BoundingRect, Text}` | `WIN Media/Ocr/mod.rs:192, 199, 160, 167, 231, 238` |
| `OcrEngine: Send + Sync` | `WIN Media/Ocr/mod.rs:153-154` |
| `SoftwareBitmap::CreateCopyFromBuffer`, `BitmapPixelFormat::Bgra8` | `WIN Graphics/Imaging/mod.rs:2295`, `:768` |
| `DataWriter::new`, `WriteBytes`, `DetachBuffer` | `WIN Storage/Streams/mod.rs:308, 352, 446` |
| `Language::CreateLanguage`, `LanguageTag`, `DisplayName` | `WIN Globalization/mod.rs:2778, 2729, 2736` |
| `Rect { X, Y, Width, Height }`, `IReference::Value` | `WIN Foundation/mod.rs:2418`, `:1311` |
| `RoInitialize`, `RO_INIT_MULTITHREADED` | `WIN Win32/System/WinRT/mod.rs:233`, `:2567` |
| Interface nulle → `Err(Error::empty())` | `WC type.rs:39-45` ; `WR error.rs:82` |
| `IVectorView::{GetAt, Size}`, `IntoIterator` (avec `unwrap`) | `WCO bindings.rs:1723, 1735, 1803` |
| `IAsyncOperation::{join, SetCompleted, GetResults, Cancel}`, `AsyncOperationCompletedHandler::new` | `WF join.rs:12`, `bindings.rs:1670, 1694, 1738, 483` |
| `HSTRING::to_string_lossy`, `From<&str>` | `WS hstring.rs:23, 129` |
| `windows::core::Error::{code, Display}` | `WR error.rs:123, 213` |
| Features `Media_Ocr`, `Graphics_Imaging`, `Storage_Streams`, `Globalization`, `Foundation_Collections`, `Win32_System_WinRT` | `windows-0.62.2/Cargo.toml` (lignes 154-221, 287, 659) |
| `accesskit_windows` dépend de `windows` 0.62 | `accesskit_windows-0.32.1/Cargo.toml` ; `Cargo.lock` |
| `egui::ProgressBar::{new, show_percentage}`, `Ui::add_enabled_ui` | `egui-0.36.2/src/widgets/progress_bar.rs:27, 69` ; `src/ui.rs:1620` |

Code SuniPDF réutilisé : `lib.rs:215,224,246` ; `pages.rs:151,246,266,282` ; `text.rs:86` ; `edit.rs:98,107,182,210-216,284-289,442,457` ; `convert.rs:162,204` ; `worker.rs:372,387,641-659` ;
`dialog.rs:72` (`show`), `:172` (`primary`), `:197` (`secondary`) ; `tools.rs:276-288` (pied de dialogue) ; `shell.rs:60-110,246-307` ; `app.rs:422-442,591-596,955,1877,1924`.

## Annexe B — Commandes réellement exécutées pour ce plan (lecture seule, aucune écriture dans le dépôt)

- Langues OCR et moteur (PowerShell + WinRT) : `OcrEngine.AvailableRecognizerLanguages` → `en-GB` ; `TryCreateFromLanguage` fr-FR → null, en-US → moteur `en-GB` ; `MaxImageDimension` = 10000 ;
  `Windows.Media.Ocr.dll` 10.0.26100.9278 ; `C:\Windows\OCR\en-gb` ; OCR d'une image 2480×3508 : 225–255 ms (1re), 56–91 ms (suivantes) ; formats Bgra8/Rgba8/Gray8 acceptés ; 10001 px → `0x80070057`.
- `Get-Command cmake,clang,ninja,vcpkg,tesseract,soffice,winword` → tous absents. `Get-WindowsCapability -Online` exige l'élévation (non exécuté avec succès).
- PDF fabriqués en Python (`-I`) puis `target/debug/sunipdf-cli.exe text|search|render` avec `SUNIPDF_PDFIUM_DIR=third_party/pdfium/win-x64/bin` : texte invisible mot par mot, extraction avec espaces, recherche multi-objets, accents WinAnsi.
- Sonde C directe de `pdfium.dll` 7881 (Python `ctypes`) : création d'un objet texte, `FPDFTextObj_SetTextRenderMode(3)`, `FPDFPageObj_GetBounds` avant/après transformation, `FPDFPage_GenerateContent`, `FPDF_SaveAsCopy` → flux décompressé contenant `3 Tr` ; rendu : 0 pixel sombre pour l'objet invisible ;
  coût de régénération 200/500 objets : 0,018 s / 0,103 s (à chaque ajout) contre 0,001 s / 0,003 s (une fois).
- API GitHub (`tesseract-ocr/tessdata_fast`, `tessdata_best`, `tessdata`) pour les tailles `eng`/`fra`/`osd` ; API crates.io pour `tesseract`, `leptess`, `tesseract-sys`, `tesseract-plumbing`, `rusty-tesseract`, `tesseract-rs`, `tesseract-static` ; README `leptess` (vcpkg) ;
  learn.microsoft.com « Language and region Features on Demand » (noms de capacité `Language.OCR~~~fr-FR~0.0.1.0`) et « Segoe Fluent Icons » (`E8FE` = Scan).
- Calcul de CER : distance de Levenshtein sur les quatre lignes de l'échantillon (EN 1/87 ; FR avec moteur anglais 25/87).
- Lecture intégrale des modules SuniPDF listés en Annexe A et des sources de `pdfium-render` 0.9.4, `windows` 0.62.2, `windows-core`, `windows-future`, `windows-collections`, `windows-strings`, `windows-result`, `egui` 0.36.2.
