# Tranche « forms » : formulaires AcroForm (lister, remplir, aplatir, importer/exporter)

Statut : plan d'implémentation (aucun code du dépôt écrit, aucune commande `cargo` lancée : la machine est partagée).
Auteur : agent forms-accessibility. Date : 2026-10-14. Base : SuniPDF 0.3.0, `main`, PDFium 7881, `pdfium-render` 0.9.4, `lopdf` 0.45.
Propriétaires de l'implémentation : moteur (`sunipdf-core`) puis interface (`apps/sunipdf-desktop`) ; le catalogue i18n est
propriété de ui-ux (D013).

Conventions de citation (toutes relues dans les sources locales, numéros de ligne de ces versions) :

| Sigle | Racine |
|---|---|
| `PR` | `C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pdfium-render-0.9.4/src` |
| `LP` | `.../lopdf-0.45.0/src` |
| `EG` | `.../egui-0.36.2/src` |
| `CORE` | `crates/sunipdf-core/src` |
| `DESK` | `apps/sunipdf-desktop/src` |

Les numéros de ligne de `CORE`/`DESK` sont ceux du 2026-10-14 ; d'autres tranches (`docs/plans/*.md`) modifient les mêmes
fichiers, il faut relire avant d'éditer (§5.12).

**Ce qui a été exécuté pour écrire ce plan.** Le code Rust n'a pas été compilé. Les comportements de PDFium ont été **mesurés** avec
un banc d'essai Python/ctypes chargeant `third_party/pdfium/win-x64/bin/pdfium.dll` (le même binaire que l'application), sur des PDF
fabriqués par le générateur de l'annexe A. Les 14 constats vérifiés sont au §2.3 (`[V1]` à `[V14]`) ; le banc d'essai n'est pas
livré : les tests Rust du §8 reprennent chaque constat comme garde-fou.

---

## 0. Décisions de conception (résumé)

1. **Lecture et écriture des champs par `lopdf`, rendu et apparences par PDFium.** `pdfium-render` 0.9.4 sait lire un champ mais
   ne sait pas en écrire un correctement (§2.3 `[V2]` à `[V4]`) : pas de régénération d'apparence, écriture sur le mauvais
   dictionnaire pour un champ à plusieurs widgets, case à cocher figée sur `/Yes`, bouton radio qui écrit « Off », **aucun
   setter pour les listes déroulantes et les zones de liste**. On écrit donc `/V` (et `/AS`, `/I`) avec `lopdf` sur l'instantané
   du document, on **supprime `/AP`** des widgets texte/liste modifiés, on recharge (`Document::restore`) puis on **force PDFium à
   régénérer et mémoriser** les apparences par un rendu minuscule (`FPDF_FFLDraw`, `[V6]`). Même mécanisme que
   `docs/plans/annotations-advanced.md` (`ensure_appearances`, constat F10) : une seule implémentation partagée.
2. **Un seul modèle de champ, construit avec `lopdf`** (`forms.rs`) : sert à lister, remplir, exporter, valider. PDFium n'est qu'un
   *oracle de test* (test unitaire de concordance, §8.1 T11).
3. **Aplatir = `FPDFPage_Flatten` via `PdfPage::flatten()`, avec trois précautions** mesurées : poser l'indicateur « Print » sur les
   annotations (sinon elles sont **supprimées sans être dessinées**, `[V8]`), générer les apparences avant (sinon un champ jamais
   rendu **disparaît**, `[V8]`), retirer `/AcroForm` du catalogue à la fin (`lopdf`). PDFium aplatit champs **et** annotations en une
   passe : l'opération s'appelle « Aplatir le formulaire et les annotations ».
4. **Documents chiffrés : remplissage refusé** (`EngineError::SecurityRestricted`), parce que le passage par `lopdf` retire
   `/Encrypt` et livrerait un fichier en clair (même garde que `annotations-advanced.md` §3, `load_for_edit`). Lecture et
   aplatissement restent possibles. **XFA : lecture seule** (`EngineError::FormXfa`). **JavaScript : jamais exécuté**, signalé.
5. **Module du moteur, pas de nouveau crate** : `crates/sunipdf-core/src/forms.rs` (+ `form_data.rs`). `docs/ARCHITECTURE.md` prévoit un
   crate `sunipdf-forms`, mais `edit`, `stamp`, `security` sont déjà des modules de `sunipdf-core` et partagent les mêmes
   types (`Document`, `PointRect`) : à valider par l'architect (question ouverte Q1).
6. **Interface** : outil « Remplir le formulaire » (surbrillance des champs + saisie en place), panneau « Champs du formulaire »
   (alternative clavier/lecteur d'écran), bandeau d'information à l'ouverture, boîte « Aplatir », export/import JSON, « Effacer les
   champs ». Tout passe par le registre de commandes, `EditOp` sous instantané (annulable), `dialog::show`.
7. **Création de champs (« Prepare a form ») : faisable avec `lopdf` seul**, sans API PDFium (§3.11, `[V11]`). Étape **optionnelle**
   (texte et case à cocher). FDF/XFDF : hors périmètre (§2.2).

---

## 1. Objectif et comportement visible

Correspondance avec la liste « All tools » de PDFCraft : **Prepare a form** (moitié « créer des champs », étape 11 optionnelle) et
**Fill & Sign** (moitié « remplir » ; la moitié « signer » est dans `docs/plans/sign-image.md`).

| Ce que l'utilisateur fait | Ce qu'il voit |
|---|---|
| Ouvre un PDF contenant un formulaire AcroForm | Bandeau bleu clair au-dessus de la page : « Ce document contient un formulaire à remplir. » + bouton « Remplir le formulaire » + « Masquer ». Aucun bandeau pour un PDF sans champ. |
| Choisit « Remplir le formulaire » (catégorie **Formulaires**, menu **Formulaires**, palette Ctrl+K, palette flottante) | Chaque widget est surligné (fond bleu 12 %, contour bleu ; **rouge** si obligatoire, **gris** si lecture seule). Le panneau « Champs du formulaire » s'ouvre à droite. Curseur main sur un champ. |
| Clique une case à cocher / un bouton radio | Basculement immédiat (coche dessinée par PDFium), une entrée d'historique « Annuler ». |
| Clique un champ texte | Un éditeur apparaît exactement sur le widget (police proportionnelle à la hauteur, `char_limit` = `MaxLen`). Entrée valide (champ sur une ligne), Échap annule, clic ailleurs ou Tab valide ; Tab ouvre le champ suivant dans l'ordre de lecture. |
| Clique une liste déroulante / une zone de liste | Petite fenêtre flottante listant les options (libellés d'affichage). Choix unique : clic = valide. Choix multiple : cases à cocher. |
| Utilise le panneau (souris, clavier, lecteur d'écran) | Liste des champs de la page (ou de tout le document), un contrôle par champ : zone de texte, case, boutons radio, liste. Chaque contrôle a un nom accessible traduit. Saisie validée à la perte de focus ou à Entrée. |
| Menu Formulaires › « Effacer les champs » | Textes vidés, cases décochées, listes désélectionnées ; champs en lecture seule et groupes radio « sans désélection » inchangés. |
| Menu Formulaires › « Aplatir le formulaire et les annotations… » | Boîte `dialog::show` : portée (toutes les pages / page courante), avertissement. Après validation : plus de champs, les valeurs sont dans le contenu, surlignages et notes aussi. Annulable (Ctrl+Z). |
| Menu Formulaires › Exporter / Importer (JSON) | Fichier `{ "nom": "valeur", … }` ; à l'import, message « N valeur(s) importée(s), M ignorée(s) ». |
| Ouvre un formulaire XFA | Bandeau : lecture seule, remplissage et aplatissement désactivés (erreur typée `form_xfa`). |
| Ouvre un formulaire à scripts (calculs, formats) | Bandeau d'avertissement : SuniPDF n'exécute pas les scripts, les champs calculés ne sont pas mis à jour. |
| Ouvre un formulaire protégé par mot de passe | Contrôles désactivés + bandeau « retirez la protection (menu Sécurité) ». |
| Enregistre, rouvre, imprime, convertit | Valeurs visibles partout : page, miniatures, vue Organiser (même rendu `FPDF_FFLDraw`) ; fichier relu identique. |

CLI : `form-fields`, `fill-form`, `flatten` (§6).

---

## 2. Faisabilité

### 2.1 Verdict : **partiel**

*Oui* pour lister, remplir (texte, case, radio, liste déroulante, zone de liste), rendre, aplatir, effacer, exporter/importer en JSON,
CLI, interface, accessibilité clavier. *Partiel* parce que six sujets sont délibérément exclus ou limités (§2.2).

### 2.2 Exclusions et limites honnêtes (à documenter dans `KNOWN_ISSUES.md`)

| Sujet | Décision | Pourquoi |
|---|---|---|
| **XFA** | Détecté (`FormKind::Xfa`, `[V10]`), champs AcroForm listés en lecture seule ; `fill_form` et `flatten` renvoient `EngineError::FormXfa` | Binaire PDFium sans XFA ni V8 (D005) ; un XFA dynamique n'a pas de page AcroForm exploitable |
| **JavaScript** (calculs `/CO`, formats `/AA`, actions `/A`) | Jamais exécuté ; `FormInfo.has_scripts` + bandeau. Les APs régénérés montrent la valeur brute | Règle CLAUDE.md (« pas de JavaScript PDF ») |
| **Documents chiffrés** | Lecture + aplatissement oui ; remplissage refusé (`SecurityRestricted`) ; `/AcroForm` conservé après aplatissement | `security::load` retire `/Encrypt` ; la réécriture du chiffrement (permissions d'origine) n'est pas faisable proprement. Levée possible plus tard : ré-appliquer `security::protect` avec les permissions lues |
| **FDF / XFDF** | **Hors périmètre.** JSON seulement. XFDF *export* optionnel (étape 12, 25 lignes sans dépendance) | FDF = syntaxe PDF binaire ; XFDF import exige un lecteur XML (`quick-xml` 0.41 est déjà dans `Cargo.lock` mais via `wayland-scanner` seulement, et son API 0.41 a changé : `Event::GeneralRef`, `PR` n'est pas concerné) : décision Q5 |
| **Signature** (`/FT /Sig`) | Listée, non remplissable (`FieldValue::None`) | Tranche `security-signatures` ; toute modification d'un document signé numériquement invalide la signature (avertissement transversal : `PdfDocument::signatures()`, `PR/pdf/document.rs:320`) |
| **Boutons-poussoirs** | Listés, aucune action (envoi, réinitialisation, JS) | Pas d'action sans JS ; pas de bouton factice : le panneau affiche « sans action » |
| Polices | Texte hors Latin-1 : PDFium choisit une police **système non incorporée** (Arial, SimSun) pour l'apparence `[V7]` ; le rendu peut varier sur une autre machine | Comportement PDFium, pas de gestion de polices dans cette tranche |
| Cases à cocher « groupes » à états distincts, radios « en unisson », `/Opt` des radios | Simplifiés : une case = un état actif lu dans `/AP /N` ; `FieldValue::Choice` d'un radio = nom d'état du widget | Cas rares |
| Texte riche (`/RV`) | Remplacé par du texte simple à l'écriture (`/RV` supprimé) | Cas rare |
| Pages avec `/Rotate` ou `/CropBox` | **Gérées** pour les champs (`display`, `[V12]`) ; les outils existants d'édition ne le sont pas (voir `edit-advanced.md`) | Cohérence avec `annotations-advanced.md` (`PageGeometry`) |
| Création de champs | Texte et case à cocher seulement (étape 11, optionnelle) | §3.11 ; liste déroulante, radio, signature : hors périmètre |

### 2.3 Constats vérifiés (PDFium 7881, banc d'essai ctypes) et conséquence de conception

Fixtures : annexe A (`acroform.pdf` : 16 widgets, 13 champs ; `acroform-noap.pdf` : sans flux d'apparence ; `acroform-xfa.pdf`).
Les décomptes de pixels sont « pixels dont R, V et B < 100 » dans un rectangle PDF, rendu à l'échelle 1,0 avec `FPDF_ANNOT`.

| # | Constat | Conséquence |
|---|---|---|
| V1 | Énumération PDFium : types 1 bouton, 2 case, 3 radio, 4 liste déroulante, 5 liste, 6 texte, 7 signature ; noms qualifiés (`adresse.rue`) ; liste déroulante : `value` = **export** (`BE`), libellé dans les options (`Belgique`) ; liste multiple : `value` = premier élément seulement ; `FPDFAnnot_GetFormFieldExportValue` ne renvoie rien pour une case/radio ; `FPDFAnnot_IsChecked` renvoie **faux** même quand `/AS` est l'état actif | Ne pas lire l'état d'une case par `is_checked()` : lire `/AS` (lopdf) ; l'énumération PDFium sert d'oracle de test uniquement |
| V2 | `PdfFormTextField::set_value` (`PR/.../field/text.rs:58-64` → `set_value_impl`, `field/private.rs:273-292`) = `FPDFAnnot_SetStringValue_str("V")` + `"M"` : **l'apparence n'est pas régénérée** (rendu `nom` : 0 px avant, 0 px après ; le fichier enregistré garde l'ancien `/AP`) | Écrire `/V` ne suffit pas : supprimer `/AP` puis forcer la régénération |
| V3 | Champ à **deux widgets** (parent portant `/T`, kids `/Parent`) : `set_value` renvoie `Ok(())` mais `GetFormFieldValue` reste vide pour les deux widgets (la valeur est écrite sur le widget, PDFium lit le parent) | `set_value` est un no-op silencieux sur les champs hiérarchiques : écrire sur le dictionnaire **de champ** |
| V4 | `PdfFormCheckboxField::set_checked` (`checkbox.rs:344-362`) écrit `/Yes` ou `/Off` en dur ; `PdfFormRadioButtonField::set_checked` (`radio.rs:87-92`) écrit la valeur **courante de `/AS`** comme valeur de groupe (pour un bouton non choisi : « Off ») ; `combo.rs` et `list.rs` n'ont **aucun setter** et `PdfFormFieldOption` (`option.rs:179-212`) n'expose pas la valeur d'export | Choisir l'état actif dans `/AP /N` (clé ≠ `Off`) ; écrire liste/combo avec lopdf |
| V5 | `FPDF_RenderPageBitmap` seul (même avec `FPDF_ANNOT`) **ne dessine aucun widget** ; ils sont dessinés par `FPDF_FFLDraw` (`PR/pdf/document/page.rs:689-702`, branche `do_render_form_data`, `render_config.rs:96` par défaut `true`, `:393`). La forme n'existe que si le document a un AcroForm (`PR/pdf/document.rs:182-184`, `form.rs:163-170`) ; toute transformation (`set_matrix`, échelle) désactive le rendu des champs (`render_config.rs:387-391`) | Garder `render_form_data(true)` + `set_target_size` (déjà le cas, `CORE/lib.rs:236-240`) ; ne jamais passer par une matrice pour rendre une page de formulaire |
| V6 | PDFium **génère l'apparence des widgets sans `/AP`** à la création de la vue de page, c'est-à-dire au premier `FPDF_FFLDraw` (un appel sur un bitmap 4×4 suffit), à partir de `/V`, `/MK` (fond, bordure), `/DA`, `/Q`, `/Opt`, `/I`, avec la police `/Helv` de `/DR` ; l'apparence générée **est mémorisée par `FPDF_SaveAsCopy`** (flux relu : `0.95 0.95 1 rg … (Hello É) Tj`). Un `/AP` existant, même périmé, est **cru** ; avec `/NeedAppearances true` PDFium régénère tout à chaque création de vue (556 → 624 px après changement de valeur) | Stratégie : supprimer `/AP` des seuls widgets modifiés (pas de `/NeedAppearances` global : il altérerait les champs non touchés et les champs à format JS) puis `ensure_appearances` |
| V7 | Édition lopdf simulée sur `acroform.pdf` : `/V` sur le parent + `/AP` retiré des deux widgets du téléphone → les deux affichent la valeur (495 px chacun) ; case `/V /Yes /AS /Yes` avec `/AP` existant → coche (20 px) ; radio `/V /carte` + échange des `/AS` → carré sur *carte* (64 px) et plus sur *virement* ; liste sans `/I` : sélection déduite de `/V` ; liste multiple `/V [..] /I [0 2]` : deux lignes surlignées ; liste déroulante : libellé affiché (« Belgique ») ; accents (É, ü) corrects ; grec et japonais rendus via polices système | Le jeu de mutations du §3.6 est suffisant |
| V8 | `PdfPage::flatten()` (`PR/pdf/document/page.rs:833-858`, `FLAT_PRINT`) : un widget **sans indicateur Print** (`/F 0`) est supprimé **sans être dessiné** (champ `ville` : 173 → 0 px) ; idem pour toute annotation créée par PDFium (`FPDFPage_CreateAnnot` donne `/F 0` : surlignage 12 529 px jaunes → 0, c'est ce que produit `add_markup`) ; avec `set_is_printed(true)` (`PR/.../annotation.rs:979`, impl `:1245`) tout est conservé ; un champ jamais rendu par `FFLDraw` et sans `/AP` disparaît ; `FLAT_NORMALDISPLAY` dessinerait le `/F 0` mais `pdfium-render` ne l'expose pas ; `/AcroForm` reste dans le catalogue et `FPDF_GetFormType` vaut encore 1 | Les trois précautions du §0.3 |
| V9 | La fonctionnalité Cargo `flatten` de `pdfium-render` (`Cargo.toml:51`) active un `flatten_page` **bouchon** : `PR/pdf/document/page/flatten.rs:4-6` = `unimplemented!()` | **Ne jamais activer** la fonctionnalité `flatten` |
| V10 | Une entrée `/XFA` dans l'AcroForm donne `FPDF_GetFormType` = 3 (`XFA_FOREGROUND`) même dans ce binaire sans XFA ; les champs AcroForm restent énumérables. `PdfForm::form_type()` (`PR/pdf/document/form.rs:187-192`) renvoie `PdfFormType::{None, Acrobat, XfaFull, XfaForeground}` (`:23-30`) | `FormKind::Xfa` sans lopdf ; `PdfDocument::form()` (`document.rs:278`) vaut `None` sans AcroForm |
| V11 | **Création** : PDF sans AcroForm, widget écrit à la main (`/FT /Tx` avec `/V`, `/DA`, `/MK`, et `/FT /Btn` avec `/V /Yes /AS /Yes`), **sans `/AP`**, AcroForm sans `/DA` ni `/DR` : reconnus, apparences générées (case : coche ✔ via `/MK /CA (4)`, états `/Yes` et `/Off` créés), mémorisées à l'enregistrement, identiques après rechargement | §3.11 faisable avec lopdf seul |
| V12 | `FPDF_PageToDevice` (`PdfPage::points_to_pixels`, `PR/pdf/document/page.rs:499-540`) donne les coordonnées d'affichage pour `/Rotate` 0/90/180/270 (90° : utilisateur (150, 696) → appareil (696, 150) pour une page affichée 792×612) ; les sorties sont entières : utiliser 8 px par point | Rectangle `display` des widgets (§3.4) ; remplaçable par `PageGeometry::rect_user_to_display` quand `annotations-advanced.md` aura atterri |
| V13 | `FPDF_GetSecurityHandlerRevision` : −1 sans chiffrement, 2 pour `encrypted-rc4.pdf`. `PdfPermissions::security_handler_revision` (`PR/pdf/document/permissions.rs:81-86`) renvoie `Err` pour les révisions 5/6 (AES-256) | Garde « non chiffré » : `matches!(.., Ok(PdfSecurityHandlerRevision::Unprotected))` ; un `Err` compte comme chiffré |
| V14 | Les masques de `PdfFormFieldFlags` non fournis par PDFium (`PR/.../field/private.rs:81, 162, 165`) sont **décalés d'un bit** par rapport à ISO 32000-1 (`TextFileSelect = 1 << 21` au lieu de `1 << 20`, `ButtonIsRadio = 1 << 16` au lieu de `1 << 15`) | Calculer nos propres masques depuis `/Ff` (§3.4) ; ne pas utiliser ces constantes |

---

## 3. API moteur (`crates/sunipdf-core`)

### 3.0 Fichiers du moteur

| Fichier | Action |
|---|---|
| `src/forms.rs` | **nouveau** : types, lecture lopdf, validation, `fill_form`, `clear_form`, `flatten`, `ensure_appearances` (si absent), création (étape 11) |
| `src/form_data.rs` | **nouveau** : JSON (export, import planifié, `info_to_json`), coercion texte → valeur (CLI), XFDF export (étape 12) |
| `src/lib.rs` | `pub mod forms; pub mod form_data;` + réexports ; `DocumentInfo.form: FormKind` (§3.3) |
| `src/error.rs` | 3 variantes (§3.1) |
| `src/pages.rs` | `refresh_page_sizes` (l. 291) met aussi à jour `info.form` ; `restore` (l. 269) passe déjà par lui |
| `src/security.rs` | `fn lopdf_err` (l. 74) devient `pub(crate)` (partagé avec `annotations-advanced`) |
| `Cargo.toml` (core) | `serde_json = { workspace = true }` (§9) |
| `tests/engine.rs`, `tests/fixtures/acroform*.pdf` | §8 |

### 3.1 `error.rs` : trois variantes typées

À ajouter à `EngineError` (`CORE/error.rs:6-25`) et à `code()` (`:27-42`), puis aux correspondances de `DESK/app.rs::error_key`
(l. 1924) et au tableau du test `every_error_key_exists_in_both_catalogs` (l. 1943) :

```rust
#[error("form field not found: {0}")]
FormFieldNotFound(String),          // code "form_field_not_found"
#[error("invalid form value: {0}")]
FormValueInvalid(String),           // code "form_value_invalid" : lecture seule, type, longueur, option inconnue, nom de champ invalide
#[error("XFA forms are not supported")]
FormXfa,                            // code "form_xfa"
```

Chiffrement : on réutilise `EngineError::SecurityRestricted` (clé `error.security` existante).

### 3.2 Types publics (`forms.rs`, réexportés par `lib.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormKind { #[default] None, AcroForm, Xfa }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind { Text, Checkbox, Radio, Combo, List, PushButton, Signature }
impl FieldKind {
    pub fn id(self) -> &'static str;                // "text" "checkbox" "radio" "combo" "list" "button" "signature"
    pub fn from_id(id: &str) -> Option<Self>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldValue {
    #[default] None,                 // bouton, signature
    Text(String),                    // champ texte
    Bool(bool),                      // case à cocher
    Choice(Vec<String>),             // radio (0..1 : nom d'état actif), liste déroulante (0..1), zone de liste (0..n) : valeurs d'EXPORT
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldOption { pub value: String /* export */, pub label: String /* affichage */ }

#[derive(Debug, Clone, PartialEq)]
pub struct FieldWidget {
    pub page: usize,                 // base 0
    pub rect: PointRect,             // /Rect, espace utilisateur de la page
    pub display: PointRect,          // espace d'AFFICHAGE (rotation + CropBox appliqués), origine bas-gauche ; c'est celui que l'UI dessine
    pub on_state: Option<String>,    // case/radio : clé de /AP /N différente de Off
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    pub name: String,                // nom qualifié « parent.enfant »
    pub kind: FieldKind,
    pub value: FieldValue,
    pub tooltip: Option<String>,     // /TU
    pub read_only: bool, pub required: bool,
    pub multiline: bool, pub password: bool, pub comb: bool, pub max_len: Option<u32>,   // texte
    pub multi_select: bool, pub editable: bool,                                           // liste / liste déroulante
    pub no_toggle_off: bool,                                                              // radio : une option reste obligatoire
    pub options: Vec<FieldOption>,   // liste déroulante / zone de liste (Opt) ; radio : une entrée par widget (value = on_state, label = on_state)
    pub widgets: Vec<FieldWidget>,   // vide = champ sans emplacement (champ masqué, orphelin après aplatissement partiel)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormInfo {
    pub kind: FormKind,
    pub has_scripts: bool,           // /CO non vide, /AA ou /A de type JavaScript sur un champ ou un widget
    pub protected: bool,             // document chiffré : lecture seule
    pub fields: Vec<FormField>,      // ordre de /Fields (profondeur d'abord)
}
impl FormInfo { pub fn field(&self, name: &str) -> Option<&FormField>; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode { Strict /* première erreur = échec, rien n'est modifié */, Lenient /* ignore et signale */ }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillReport {
    pub applied: Vec<String>,
    pub skipped_unknown: Vec<String>,
    pub skipped_invalid: Vec<(String, String)>,     // (nom, raison en anglais)
}
```

### 3.3 `FormKind` du document et `DocumentInfo`

```rust
// forms.rs
pub(crate) fn kind_of(doc: &PdfDocument<'_>) -> FormKind {
    match doc.form().map(|f| f.form_type()) {                  // PR/pdf/document.rs:278, form.rs:187
        Some(PdfFormType::Acrobat) => FormKind::AcroForm,
        Some(PdfFormType::XfaFull | PdfFormType::XfaForeground) => FormKind::Xfa,
        _ => FormKind::None,
    }
}
impl Document<'_> {
    pub fn form_kind(&self) -> FormKind { kind_of(&self.inner) }
    fn is_unprotected(&self) -> bool {
        // PR/pdf/document/permissions.rs:81-86 ; Err (révisions 5/6) compte comme protégé
        matches!(self.inner.permissions().security_handler_revision(), Ok(PdfSecurityHandlerRevision::Unprotected))
    }
}
```

`DocumentInfo` (`CORE/lib.rs:56-60`) gagne `pub form: FormKind` ; renseigné dans `wrap` (l. 159-190, un seul constructeur : `grep` montre
`DocumentInfo {` uniquement là) et dans `refresh_page_sizes` (`CORE/pages.rs:291`). Coût nul (un appel de lecture). L'interface s'en sert
pour le bandeau et pour griser les commandes sans relire tout le formulaire. Après un aplatissement complet, `form` redevient `None`.

### 3.4 Lecture : `Document::form_info`

```rust
impl Document<'_> {
    /// Instantané → lopdf (lecture seule) → modèle ; les rectangles d'affichage viennent de PDFium.
    pub fn form_info(&self) -> Result<FormInfo, EngineError>;
}
```

Algorithme :

1. `kind = self.form_kind()` ; si `None` → `FormInfo::default()` (aucune analyse, aucun instantané : c'est le cas courant).
2. `bytes = self.snapshot()?` (`CORE/pages.rs:243`) ; `ld = crate::security::load(&bytes, self.password())?` (déchiffre si besoin, lecture seule :
   `CORE/security.rs:78-95`).
3. `model = build_model(&ld)` (ci-dessous) → `Vec<Raw>` + `has_scripts`.
4. `display` : pour chaque page contenant des widgets, **un** `self.page_handle(i)?` puis, par widget, `display_rect(&page, rect)` :

```rust
/// [V12] FPDF_PageToDevice est entière : on travaille à 8 px par point.
fn display_rect(page: &PdfPage<'_>, r: PointRect) -> Option<PointRect> {
    const S: f32 = 8.0;
    let (w, h) = (page.width().value, page.height().value);                    // PR page.rs:248,255 : dimensions AFFICHÉES
    let cfg = PdfRenderConfig::new().set_target_size((w * S).round() as Pixels, (h * S).round() as Pixels);   // render_config.rs:187 ; Pixels = i32 (bitmap.rs:55)
    let a = page.points_to_pixels(PdfPoints::new(r.left), PdfPoints::new(r.bottom), &cfg).ok()?;              // page.rs:499
    let b = page.points_to_pixels(PdfPoints::new(r.right), PdfPoints::new(r.top), &cfg).ok()?;
    let (x1, x2) = (a.0.min(b.0) as f32 / S, a.0.max(b.0) as f32 / S);
    let (y1, y2) = (a.1.min(b.1) as f32 / S, a.1.max(b.1) as f32 / S);          // origine HAUT-gauche en sortie
    Some(PointRect { left: x1, right: x2, top: h - y1, bottom: h - y2 })        // retour à l'origine bas-gauche
}
```

   Repli si la conversion échoue : `display = rect`. Quand `annotations-advanced.md` aura atterri (`geometry.rs`, `PageGeometry::rect_user_to_display`),
   remplacer le corps par un appel à `page_geometry(page)?.rect_user_to_display(rect)` (mêmes résultats : constat F8 de ce plan).

5. `protected = !self.is_unprotected()`.

**Construction du modèle lopdf** (interne, `forms.rs`) :

```rust
struct RawWidget { id: ObjectId, on_state: Option<String>, page: Option<usize>, rect: Option<PointRect> }
struct Raw { field_id: ObjectId /* dictionnaire qui porte /V */, widgets: Vec<RawWidget>, view: FormField }
#[derive(Clone, Default)]
struct Inherited { ft: Option<Vec<u8>>, ff: u32, v: Option<Object>, opt: Option<Object>, max_len: Option<u32> }
```

Appels lopdf utilisés : `Document::catalog()` (`LP/document.rs:516`), `get_dictionary` (`:202`), `dereference` (`:147`), `get_pages` (`:533`),
`Dictionary::{get, has, iter}` (`LP/object.rs:352, 348, 400`), `Object::{as_name, as_str, as_array, as_dict, as_i64, as_float, as_reference}`
(`LP/object.rs:145-268`), `lopdf::decode_text_string` (`LP/common_data_structures/mod.rs:22`, réexporté `LP/lib.rs:43`).
**Ne pas utiliser** `get_page_annotations` (`LP/document.rs:747`) : il ne rend pas les identifiants.

```rust
fn deref<'a>(d: &'a lopdf::Document, o: &'a Object) -> Option<&'a Object> { d.dereference(o).ok().map(|(_, o)| o) }
fn get<'a>(d: &'a lopdf::Document, dict: &'a Dictionary, k: &[u8]) -> Option<&'a Object> { dict.get(k).ok().and_then(|o| deref(d, o)) }

// Masques ISO 32000-1 (tableaux 221, 226, 228, 230) : position de bit n -> 1 << (n - 1). [V14]
const FF_READ_ONLY: u32 = 1 << 0;   const FF_REQUIRED: u32 = 1 << 1;
const FF_MULTILINE: u32 = 1 << 12;  const FF_PASSWORD: u32 = 1 << 13;  const FF_FILE_SELECT: u32 = 1 << 20;  const FF_COMB: u32 = 1 << 24;
const FF_NO_TOGGLE_OFF: u32 = 1 << 14; const FF_RADIO: u32 = 1 << 15;  const FF_PUSHBUTTON: u32 = 1 << 16;
const FF_COMBO: u32 = 1 << 17;      const FF_EDIT: u32 = 1 << 18;      const FF_MULTI_SELECT: u32 = 1 << 21;

impl Inherited {   // héritage /FT /Ff /V /Opt /MaxLen du parent vers l'enfant (ISO 32000-1 §12.7.3.1)
    fn merged(&self, d: &lopdf::Document, dict: &Dictionary) -> Inherited { /* chaque clé : valeur du dictionnaire sinon celle du parent */ }
}

fn kind_of_field(inh: &Inherited) -> Option<FieldKind> {
    Some(match inh.ft.as_deref()? {
        b"Tx" => FieldKind::Text,
        b"Btn" if inh.ff & FF_PUSHBUTTON != 0 => FieldKind::PushButton,
        b"Btn" if inh.ff & FF_RADIO != 0 => FieldKind::Radio,
        b"Btn" => FieldKind::Checkbox,
        b"Ch" if inh.ff & FF_COMBO != 0 => FieldKind::Combo,
        b"Ch" => FieldKind::List,
        b"Sig" => FieldKind::Signature,
        _ => return None,                                   // /FT inconnu : champ ignoré (journal), jamais de panique
    })
}

/// État actif d'une case/radio : première clé de /AP /N différente de Off.
fn on_state(d: &lopdf::Document, w: &Dictionary) -> Option<String> {
    let n = get(d, get(d, w, b"AP")?.as_dict().ok()?, b"N")?.as_dict().ok()?;   // N est un flux (texte) -> as_dict échoue -> None
    n.iter().map(|(k, _)| k).find(|k| k.as_slice() != b"Off").map(|k| String::from_utf8_lossy(k).into_owned())
}

/// Widget -> page (base 0) : scan de /Annots de chaque page ; ne pas se fier à /P.
fn page_of_annots(d: &lopdf::Document) -> HashMap<ObjectId, usize> { /* get_pages() (base 1) ; /Annots tableau direct ou indirect ; entrées Reference */ }

fn walk(d: &lopdf::Document, id: ObjectId, prefix: &str, parent: &Inherited, ctx: &mut Ctx, depth: usize) {
    if depth > 32 || !ctx.seen.insert(id) { return; }                         // cycles : PDF hostile
    let Ok(dict) = d.get_dictionary(id) else { return };
    let inh = parent.merged(d, dict);
    let name = join_name(prefix, text(d, dict, b"T"));                        // decode_text_string
    let kids: Vec<ObjectId> = /* /Kids : tableau direct ou indirect, entrées Reference */;
    let is_field = |k: ObjectId| d.get_dictionary(k).is_ok_and(|kd| kd.has(b"T") || kd.has(b"Kids"));
    if kids.iter().any(|&k| is_field(k)) {                                    // non terminal : recurse
        for k in kids { walk(d, k, &name, &inh, ctx, depth + 1); }
        return;
    }
    let widget_ids = if kids.is_empty() { if is_widget(dict) { vec![id] } else { vec![] } } else { kids };
    if let Some(raw) = terminal(d, id, name, &inh, widget_ids, ctx) { ctx.out.push(raw); }
}
```

`terminal` calcule : `kind_of_field`, les drapeaux, `tooltip` (`/TU`), `max_len`, `options` (`/Opt` : chaîne → `{value = label}` ; tableau
`[export libellé]` → paire ; décodage `decode_text_string`), les widgets (`rect_of` : `/Rect` normalisé min/max ; page par `page_of_annots` ; `on_state`) et la valeur :

| Genre | Valeur lue |
|---|---|
| Texte | `V` chaîne ou flux → `Text(decode)` ; absent → `Text("")` |
| Case | `AS` du premier widget ≠ `Off` ; sinon `V` nom ≠ `Off` → `Bool` (**jamais `IsChecked`**, `[V1]`) |
| Radio | `V` nom ≠ `Off` → `Choice([nom])` ; sinon premier widget dont `AS` ≠ `Off` ; sinon `Choice([])` |
| Liste déroulante / zone de liste | `V` chaîne → `[v]` ; tableau de chaînes → toutes ; absent → `[]` |
| Bouton / signature | `None` |

`has_scripts` : `/CO` de l'AcroForm non vide, ou un `/AA` (toute action de sous-type `/S /JavaScript`), ou `/A /S /JavaScript`, sur un champ ou un widget.
Les widgets dont la page est introuvable sont **exclus de `view.widgets`** mais conservés dans `Raw.widgets` (l'écriture les met à jour quand même).

### 3.5 Validation (partagée par l'écriture, l'import JSON et la CLI)

```rust
impl FormField {
    /// Raison en anglais (jamais affichée telle quelle) ou Ok. Ne modifie rien.
    pub fn validate(&self, v: &FieldValue) -> Result<(), String>;
    /// Option par valeur d'export, à défaut par libellé.
    pub fn option(&self, x: &str) -> Option<&FieldOption>;
}
```

Règles : `read_only` → « read-only » ; `Text` : mot de passe refusé (une valeur de champ mot de passe ne doit jamais être stockée,
`PR/.../private.rs` doc de `TextPassword`), retour à la ligne seulement si `multiline`, longueur ≤ `max_len` (en caractères) ;
`Checkbox` : `Bool` ; `Radio` : `[]` seulement si `!no_toggle_off`, `[x]` avec `x` = `on_state` d'un widget ; `Combo` : `[]`, ou `[x]` avec option
connue (export ou libellé) ou `editable` ; `List` : toutes les valeurs connues, une seule si `!multi_select` ; bouton et signature :
« not fillable » ; type de valeur incompatible : « value type does not match the field type ».

### 3.6 Écriture : `Document::fill_form`

```rust
impl Document<'_> {
    pub fn fill_form(&mut self, values: &[(String, FieldValue)], mode: FillMode) -> Result<FillReport, EngineError>;
}
```

1. **Gardes** (avant toute lecture) : `FormKind::Xfa` → `FormXfa` ; `!self.is_unprotected()` → `SecurityRestricted`.
2. `bytes = self.snapshot()?` ; `ld = load_for_edit(&bytes)?` (`lopdf::Document::load_mem` puis `is_encrypted()` → `SecurityRestricted`, **sans** décryptage ; définir ici si
   `annot_write::load_for_edit` n'existe pas encore, à factoriser) ; `raws = build_model(&ld)`.
3. **Validation complète avant toute mutation** : pour chaque `(nom, valeur)` : champ introuvable → `Strict` : `Err(FormFieldNotFound)` / `Lenient` :
   `skipped_unknown` ; `validate` en échec → `Strict` : `Err(FormValueInvalid("nom: raison"))` / `Lenient` : `skipped_invalid`. Aucun effet de bord en cas d'erreur.
4. **Mutations** (`apply`, lopdf) :

| Genre | Mutations |
|---|---|
| Texte | `V = lopdf::text_string(s)` (`LP/common_data_structures/mod.rs:9` : ASCII littéral, sinon UTF-16BE avec BOM) **sur `field_id`** ; supprimer `RV` ; supprimer `AP` de **chaque widget** |
| Case | `on = on_state du premier widget ou "Yes"` ; `V = Name(on ou "Off")` ; `AS = Name(...)` sur **chaque widget** (son propre `on_state` sinon `on`) ; `AP` conservé |
| Radio | `V = Name(choisi)` ou `Name("Off")` ; `AS` de chaque widget = `Name(choisi)` si son `on_state` correspond, sinon `Name("Off")` ; `AP` conservé |
| Liste déroulante | `V = text_string(export de l'option)` (ou le texte tel quel si `editable` et inconnu) ; supprimer `I` ; supprimer `AP` des widgets |
| Zone de liste | `V` = tableau de `text_string` si `multi_select`, sinon chaîne ; `I` = tableau d'entiers triés des indices ; vide → supprimer `V` et `I` ; supprimer `AP` des widgets |

   Attention : `impl From<&str> for Object` produit un **Name** (`LP/object.rs:106-110`) ; les chaînes PDF passent par `text_string` ou `Object::string_literal` (`:137`).
   `LP/document.rs:178` `get_object_mut` suit les références ; utiliser `get_dictionary_mut` (`:207`).
5. `out = crate::security::save(ld)?` (`CORE/security.rs:96`) ; `self.restore(out)?` (`CORE/pages.rs:269` : charge avant de remplacer ; en cas d'échec le document reste intact) ;
   pour chaque page touchée : `self.ensure_appearances(page)?` (§3.7).
6. Renvoie `FillReport` (`applied` = noms appliqués).

Instantané/annulation : l'appelant (`State::edit`, `DESK/worker.rs:372`) prend déjà l'instantané avant `apply` et le restaure sur erreur ; la CLI n'en a pas besoin.

### 3.7 Apparences : `ensure_appearances`

```rust
impl Document<'_> {
    /// Rend la page en ~8 px de large : PDFium crée la vue de page et génère puis mémorise les /AP manquants [V6].
    pub fn ensure_appearances(&self, page_index: usize) -> Result<(), EngineError> {
        let w = self.page_size(page_index)?.width_pt;
        self.render_page(page_index, 8.0 / w).map(|_| ())          // CORE/lib.rs:224-247 : render_form_data(true) + render_annotations(true)
    }
}
```

Définir **une seule fois** (même nom et même corps que `docs/plans/annotations-advanced.md` §3.3 : le premier atterri la crée). Aucun rendu n'est réutilisé : le coût est négligeable.

### 3.8 `clear_form`

```rust
pub fn clear_form(&mut self) -> Result<usize, EngineError>;       // nombre de champs modifiés
```
Construit `Text("")`, `Bool(false)`, `Choice([])` pour les champs ni en lecture seule, ni bouton, ni signature, ni radio `no_toggle_off`, puis `fill_form(.., Lenient)`.

### 3.9 Aplatissement : `Document::flatten`

```rust
pub fn flatten(&mut self, pages: Option<&[usize]>) -> Result<usize, EngineError>;   // nombre d'annotations traitées
```

```rust
if self.form_kind() == FormKind::Xfa { return Err(EngineError::FormXfa); }
let total = self.page_count();
let targets: Vec<usize> = pages.map_or_else(|| (0..total).collect(), <[usize]>::to_vec);
if let Some(&bad) = targets.iter().find(|&&p| p >= total) { return Err(EngineError::PageOutOfRange(bad)); }
let mut count = 0;
for &p in &targets {
    self.ensure_appearances(p)?;                                   // sinon un champ sans /AP disparaît [V8]
    let mut page = self.page_handle(p)?;                           // CORE/pages.rs:282
    for mut a in page.annotations().iter() {                       // PR/.../annotations.rs:139
        count += 1;
        if !a.is_hidden() && !a.is_printed() { a.set_is_printed(true)?; }   // PR annotation.rs:954, 970, 979 : FLAT_PRINT supprime sans dessiner [V8]
    }
    page.flatten()?;                                               // PR page.rs:833-858 : FPDFPage_Flatten(FLAT_PRINT) + regenerate_content + reload ; PageFlattenFailure -> EngineError::Other
}
if targets.len() == total { self.drop_acroform()?; }
Ok(count)
```

`drop_acroform` : si `form_kind() != None` et `is_unprotected()` : instantané → `load_for_edit` → `catalog_mut()` (`LP/document.rs:525`) `.remove(b"AcroForm")` → `prune_objects()` (`LP/processor.rs:66`, déjà utilisé par `strip_metadata`) → `save` → `restore`. Document chiffré : on garde `/AcroForm`
(les champs deviennent des champs sans widget). `?` convertit `PdfiumError` en `EngineError` (`CORE/error.rs:44-62`). **Ne jamais activer la
fonctionnalité Cargo `flatten`** (`[V9]`). Aplatissement partiel : les champs de la page aplatie restent dans `/Fields` sans widget (`widgets` vide) ; documenté.

### 3.10 JSON (`form_data.rs`)

```rust
pub fn values_to_json(info: &FormInfo) -> serde_json::Value;       // {nom: valeur} ; boutons/signatures omis ; homonymes : le premier gagne
pub fn info_to_json(info: &FormInfo) -> serde_json::Value;         // structure complète du §6 (pages en base 1)
#[derive(Debug, Default, PartialEq)]
pub struct ImportPlan { pub values: Vec<(String, FieldValue)>, pub skipped_unknown: Vec<String>, pub skipped_invalid: Vec<(String, String)> }
pub fn plan_from_json(info: &FormInfo, json: &serde_json::Value) -> Result<ImportPlan, EngineError>;   // non-objet -> FormValueInvalid
impl FormField { pub fn coerce_text(&self, text: &str) -> Result<FieldValue, EngineError>; }              // CLI --set
```

Correspondance valeur ↔ JSON : texte → chaîne ; case → booléen ; radio, liste déroulante → chaîne ou `null` ; zone de liste à choix multiple → tableau,
sinon chaîne. À l'import : `null` = vide ; case acceptant `true/false`, `"true" "yes" "on" "oui" "1"` / `"false" "no" "off" "non" "0" ""` ; chaîne ou tableau
pour les choix ; nombre → texte. Chaque valeur passe par `validate` : l'import ne peut donc pas échouer plus tard en `Strict`.
`coerce_text` : texte → `Text` ; case → booléen ; radio/liste déroulante → `Choice([texte])` (`""` = `[]`) ; zone de liste → découpe sur `;`.

### 3.11 Création de champs (étape 11, optionnelle) : texte et case à cocher

**Faisable sans API PDFium** : `pdfium-render` ne sait pas créer de widget (`PdfPageAnnotations::create_*`, `PR/.../annotations.rs:200-366` : libre-texte, surlignage, encre, lien, popup, carré, ondulé, tampon, barré, note, souligné ; aucun widget), PDFium non plus (`FPDFPage_CreateAnnot` ne crée pas de widget). Mais `[V11]` : un widget écrit en dictionnaire, **sans `/AP`**, est
reconnu, reçoit son apparence de PDFium (case : états `/Yes` et `/Off` créés) et la mémorise. Il suffit d'écrire le dictionnaire avec `lopdf`.

```rust
pub enum NewFieldKind { Text, Checkbox }
pub struct NewField { pub kind: NewFieldKind, pub name: String, pub rect: PointRect /* espace d'AFFICHAGE */, pub multiline: bool, pub required: bool }
impl Document<'_> { pub fn add_form_field(&mut self, page_index: usize, f: &NewField) -> Result<(), EngineError>; }
```

1. Gardes (`FormXfa`, chiffré → `SecurityRestricted`) ; nom : non vide, sans `.` (créerait une hiérarchie), absent de `build_model` → sinon `FormValueInvalid` ; rectangle ≥ 4 pt de côté.
2. Rectangle d'affichage → espace utilisateur (inverse de §3.4 : `PageGeometry::rect_display_to_user` si disponible, sinon `FPDF_DeviceToPage` via `PdfPage::pixels_to_points`, `PR/.../page.rs:465-495`).
3. Dictionnaire widget (aucune clé `/AP`) :
   * texte : `Type /Annot, Subtype /Widget, FT /Tx, T text_string(nom), F 4, Rect [l b r t], P <page>, DA (/Helv 12 Tf 0 g), MK << /BC [0 0 0] /BG [1 1 1] >>`, `Ff` = `4096` si multiligne `| 2` si obligatoire ;
   * case : `FT /Btn, T, F 4, Rect, P, V /Off, AS /Off, DA (/ZaDb 0 Tf 0 g), MK << /BC [0 0 0] /CA (4) >>`, `Ff` = `2` si obligatoire.
4. `id = ld.add_object(dict)` (`LP/creator.rs:21`) ; ajouter `Reference(id)` à `/Annots` de la page (tableau direct ou indirect ; créer si absent : même utilitaire `annots_mut` que `annotations-advanced.md` §3) ; ajouter à `/AcroForm /Fields` (créer l'AcroForm si absent : `Fields`, `DA (/Helv 0 Tf 0 g)`, `DR << /Font << /Helv <réf. police Helvetica WinAnsi> >> >>`, puis `catalog_mut().set("AcroForm", Reference(id))` ; `[V11]` : PDFium se passe de `DA`/`DR`, mais Acrobat les attend).
5. `save` → `restore` → `ensure_appearances(page)`.

Test T12 (§8.1). Interface : outils `ToolAddTextField` / `ToolAddCheckbox` (glissement d'un rectangle comme `Tool::Rectangle`, puis boîte « Nouveau champ » : nom, plusieurs lignes, obligatoire).

### 3.12 XFDF en export (étape 12, optionnelle)

`form_data::export_xfdf(info: &FormInfo) -> String` : `<?xml ...?><xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve"><fields>` + un `<field name="…"><value>…</value></field>` par champ (échappement `& < > "` ; hiérarchie aplatie en noms qualifiés ; case : `Yes`/`Off` ; choix multiple : un `<value>` par option) + `</fields></xfdf>`. Sans dépendance. L'import XFDF reste hors périmètre (§2.2).

---

## 4. Worker (`apps/sunipdf-desktop/src/worker.rs`)

| Élément | Ajout | Lieu (ligne actuelle) |
|---|---|---|
| `Request::FormInfo` | lecture du formulaire du document actif | `Request` (l. 178) ; traitement dans `run` près de `Request::Annotations` (l. 795) |
| `Response::FormInfo(FormInfo)` | réponse | `Response` (l. 238) |
| `EditOp::FillForm { values: Vec<(String, FieldValue)> }` | `Strict`, annulable | `EditOp` (l. 76) ; `apply` (l. 415) : `doc.fill_form(&values, FillMode::Strict).map(\|_\| ())` |
| `EditOp::ClearForm` | | `apply` : `doc.clear_form().map(\|_\| ())` |
| `EditOp::FlattenForm { pages: Option<Vec<usize>> }` | | `apply` : `doc.flatten(pages.as_deref()).map(\|_\| ())` |
| `EditOp::AddFormField { page: usize, field: NewField }` (étape 11) | | `apply` : `doc.add_form_field(page, &field)` |
| `BatchOp::ExportFormData { out: PathBuf }` | écrit le JSON des valeurs, ne modifie pas le document | `BatchOp` (l. 31) ; `run_batch` (l. 559) : `doc.form_info()` → `form_data::values_to_json` → `serde_json::to_string_pretty` → `write_atomic` |
| `BatchResult::FormExported { out: PathBuf, count: usize }` | | `BatchResult` (l. 40) ; `on_batch_done` (`DESK/tools.rs`) |

```rust
Request::FormInfo => {
    let Some(doc) = &state.document else { continue };
    Response::FormInfo(doc.form_info().unwrap_or_else(|e| {
        log::warn!("lecture du formulaire impossible : {e}");
        FormInfo::default()                       // évite une boucle de relectures côté interface
    }))
}
```

L'import JSON se fait **côté interface** (fichier minuscule, fonction pure) : lecture + `plan_from_json(&info, &json)` sur le `FormInfo` déjà en mémoire, puis
`EditOp::FillForm { values: plan.values }` ; le message d'état utilise `plan.skipped_*` (pas de variante de worker supplémentaire).
`State::edit` (l. 372) est inchangé : une opération = un instantané = un niveau d'annulation.

---

## 5. Interface

### 5.1 Commandes (`CORE/command.rs`)

Variantes de `Command` (l. 4-69) et entrées `cmd!` de `REGISTRY` **après `PageNumbers`** (l'ordre du tableau est celui des menus) ; nouveau `Menu::Forms`
(enum `Menu`, l. 71-80) :

| Variante | id | Clé du libellé | Raccourci (clé) | Document requis | Menu |
|---|---|---|---|---|---|
| `ToolFillForm` | `forms.fill` | `menu.forms.fill_tool` | aucun | oui | `Forms` |
| `ToggleForms` | `view.forms` | `menu.forms.panel` | `shortcut.forms_panel` = **Ctrl+Maj+F** | oui | `Forms` |
| `ClearForm` | `forms.clear` | `menu.forms.clear` | aucun | oui | `Forms` |
| `FlattenForm` | `forms.flatten` | `menu.forms.flatten` | aucun | oui | `Forms` |
| `ExportFormData` | `forms.export` | `menu.forms.export` | aucun | oui | `Forms` |
| `ImportFormData` | `forms.import` | `menu.forms.import` | aucun | oui | `Forms` |
| (étape 11) `ToolAddTextField`, `ToolAddCheckbox` | `forms.add_text`, `forms.add_checkbox` | `menu.forms.add_text`, `menu.forms.add_checkbox` | aucun | oui | `Forms` |

Raccourcis : `F` (annotations-advanced), `K` (bookmarks-links), `S` (sign-image), `F5` (continuous-view) sont pris par d'autres plans ; l'outil n'a pas de touche (accessible par le
panneau d'outils, la palette Ctrl+K et la palette flottante). **Ctrl+Maj+F** pour le panneau : dans `handle_input` (`DESK/app.rs:576`), placer
`key(COMMAND | SHIFT, Key::F, Command::ToggleForms)` **avant** `key(COMMAND, Key::F, Command::Find)` (l'API `consume_key` ignore les Maj/Alt en trop, comme pour Ctrl+Maj+S / Ctrl+S existants).
Le test `every_command_variant_is_registered` compte 7 commandes dans `Menu::File` : ne rien ajouter à ce menu. La CLI `commands` et la palette lisent `REGISTRY` sans autre changement.

Obligatoire à chaque commande (sinon erreur de compilation ou panique `expect("toute commande est enregistrée")`, `command.rs:614`) :
`Command` + `REGISTRY` ; `command_icon` (`DESK/shell.rs:246`, `match` exhaustif) ; `execute` (`DESK/app.rs:793`, `match` exhaustif) ; `command_color` (`shell.rs:150`, `match` exhaustif sur `Menu` : ajouter `Menu::Forms`).

Icônes (`DESK/icons.rs::glyph`, l. 12-70) : nouvelle constante `FORM = "\u{E73A}"` (case cochée de Segoe MDL2 / Fluent ; `RECT = E739` est la case vide déjà utilisée ; **à confirmer par capture**, repli texte « ☑ ») pour `ToolFillForm`
et `ToggleForms` ; `ClearForm` → `ERASE`, `FlattenForm` → `COMBINE`, `ExportFormData` → `SAVE_AS`, `ImportFormData` → `OPEN` (constantes existantes). Couleur : nouvelle `theme::CAT_FORMS = #EA580C` (`DESK/theme.rs`, après `CAT_VIEW`) renvoyée pour `Menu::Forms`
(`tool-hub.md` prévoit `65A30D` pour « Formulaire » ; reprendre sa couleur si `tool-hub` a atterri).

### 5.2 Panneaux d'outils et menus (code actuel)

* `ToolTab` (`DESK/shell.rs:12-24`) : variante `Forms`, `ALL: [ToolTab; 8]`, `label_key` → `toolbar.forms`, `commands()` :
  `[ToolFillForm, ToggleForms, ClearForm, FlattenForm, ExportFormData, ImportFormData, Undo, Redo, SaveAs]` ; ajouter `ToolFillForm` et `FlattenForm` à `ToolTab::All` (l. 46-80).
* Bouton Menu (`shell.rs:468`) : `self.command_menu(ctx, ui, "menu.forms", Menu::Forms, &[Command::FlattenForm]);` entre « Pages » et « Convertir ».
* Rail droit (`right_rail`, `shell.rs` ~560) : un bouton `glyph::FORM` (`selected(self.forms.panel_open)`), info-bulle `menu.forms.panel`, `execute(ToggleForms)`.

### 5.3 État (`DESK/forms.rs`, nouveau, déclaré dans `main.rs`)

```rust
#[derive(Default)]
pub struct FormsState {
    pub panel_open: bool,                    // préservé à la fermeture/échange d'onglet, comme comments_open (tabs.rs:60-70, 79-91)
    pub scope_all: bool,                     // panneau : tout le document (sinon page courante)
    pub info: Option<FormInfo>,
    pub stale: bool, pub requested: bool,    // relecture du formulaire
    pub order: Vec<(usize, usize)>,          // (champ, widget) en ordre de lecture : page, haut -> bas, gauche -> droite (fonction pure `reading_order`)
    pub drafts: HashMap<String, String>,     // saisies du panneau non validées
    pub editing: Option<InPlace>,            // éditeur sur la page : { field: String, widget: usize, text: String, multiline: bool, max_len: Option<u32>, focus: bool }
    pub popup: Option<ChoicePopup>,          // { field_idx, anchor: egui::Pos2 }
    pub keep_texture: bool,                  // voir 5.7
    pub bar_dismissed: bool,
    pub flatten_open: bool, pub flatten_all_pages: bool,   // `flatten_all_pages` : défaut true (Default::default() donne false : initialiser à true dans `FlattenForm`)
}
```

Branché dans `SuniApp` (`app.rs:82`, champ `pub forms: crate::forms::FormsState`), dans `TabState` (`tabs.rs:9-23`) avec `take_tab`/`put_tab` (même motif que `annotate`/`docinfo` : `std::mem::take`, `panel_open` préservé),
dans `close_document` (`app.rs:239` : `self.forms = Default::default()` en gardant `panel_open`).

Fonctions pures testables sans fenêtre : `reading_order(&FormInfo) -> Vec<(usize,usize)>`, `kind_key(FieldKind) -> &'static str` (`forms.kind.<id>`), `draft_to_value(&FormField, &str) -> FieldValue`.

### 5.4 Relecture du formulaire (motif de `refresh_annotations_if_needed`, `annotate.rs`)

* `DocumentChanged` (`app.rs:422`), `Opened` (l. 340) et `on_resumed` (`tabs.rs:168`) : `self.forms.stale = true`.
* `refresh_forms_if_needed()` appelée depuis `document_area` : si `(panel_open || outil == FillForm)` et `info_form != None` et `stale && !requested` → `Request::FormInfo`, `requested = true`, `stale = false`.
* `Response::FormInfo(info)` → `on_form_info` : `requested = false`, `order = reading_order(&info)`, nettoie `drafts` des champs dont le brouillon égale la valeur du modèle, range `info`.
* Les réponses arrivent dans l'ordre : un `FormInfo` demandé avant une modification est suivi de son `DocumentChanged`, donc d'une nouvelle relecture : jamais de donnée périmée durablement.

### 5.5 Page : surbrillance et saisie en place

**Intégration dans `document_area`** (`app.rs:1058`, fermeture `ScrollArea` l. ~1110-1186) : après `paint_object_selection` (l. 1144) et **après** la création de `interact` (l. 1145, les rectangles de champs doivent être créés *après* pour avoir
la priorité de clic), appeler :

```rust
// variables locales déclarées avant la fermeture (comme `drag_event`)
let mut form_hit: Option<(usize, usize)> = None;
let mut page_rect_out: Option<egui::Rect> = None;
...
page_rect_out = Some(page_rect);
self.forms_overlay(ui, &painter, page_rect, page_index, size.height_pt, &mut form_hit);
```

et après la fermeture, à côté de `match drag_event` (l. 1187) : `if let Some((f, w)) = form_hit { self.forms_activate(ctx, f, w); }`,
`self.forms_editor_area(ctx, page_rect_out)`, `self.forms_choice_popup(ctx, page_rect_out)`. `forms_overlay(&self, …)` est **auto-contenue et par page** pour survivre à la réécriture de `document_area` par
`continuous-view.md` (plusieurs pages visibles : l'appeler une fois par page visible avec son `page_rect`).

`forms_overlay` (actif si `annotate.tool == Tool::FillForm`) : pour chaque `(champ, widget)` de la page, `r = to_screen(widget.display)` avec le même changement de repère que `paint_selection`
(`page_rect.min + vec2(left * k, (page_h_pt - top) * k)`, `k = effective_zoom * SCREEN_PER_PDF_POINT`) ; `painter.rect_filled(r, 2.0, couleur.gamma_multiply(0.12))` + `rect_stroke(.., StrokeKind::Inside)`
(bleu `theme::BLUE`, rouge `theme::ERROR` si `required`, gris si `read_only`) ; puis

```rust
let resp = ui.interact(r, egui::Id::new(("form-widget", fi, wi)), egui::Sense::click());     // EG ui.rs:907 ; Sense::click() = CLICK | FOCUSABLE (EG sense.rs)
resp.widget_info(|| egui::WidgetInfo::labeled(widget_type, !read_only, label));              // EG response.rs:869 ; data/output.rs:658
if !read_only && !protected { resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand); }
if resp.clicked() { *hit = Some((fi, wi)); resp.surrender_focus(); }                         // EG response.rs:382 : sinon les raccourcis clavier globaux restent bloqués (handle_input retourne si un widget a le focus)
```

`widget_type` : case → `WidgetType::Checkbox`, radio → `RadioButton`, liste déroulante → `ComboBox`, texte → `TextEdit`, autre → `Button`. `label` = `tr_args("a11y.forms.widget" | "a11y.forms.widget_value", …)`.
Le curseur : dans `page_cursor` (`editor.rs:186`) ajouter l'arme `Tool::FillForm => CursorIcon::Default` (les rectangles posent eux-mêmes la main).

`forms_activate(ctx, f, w)` :

| Genre | Action |
|---|---|
| Case | `commit_form(nom, Bool(!coché))` |
| Radio | `commit_form(nom, Choice([widget.on_state]))` (sans effet si déjà choisi et `no_toggle_off`) |
| Texte (modifiable) | `forms.editing = Some(InPlace { … text: valeur courante, focus: true })` |
| Liste déroulante / zone de liste | `forms.popup = Some(ChoicePopup { field_idx, anchor: r.left_bottom() })` |
| Bouton, signature, lecture seule, document protégé, XFA | `status_msg = tr("status.form_unsupported_widget")` |

`forms_editor_area(ctx, page_rect)` : `egui::Area::new(Id::new("form-editor")).fixed_pos(r.min).order(egui::Order::Foreground).show(ctx, …)` (`EG containers/area.rs:235,277`)
contenant `egui::TextEdit::singleline` ou `multiline(..).desired_rows(..)` (`EG widgets/text_edit/builder.rs:113,123`) avec `.char_limit(max_len)` (`:389`), police `FontId::proportional((r.height() * 0.7).clamp(10.0, 28.0))`,
`.desired_width(r.width())`, `request_focus()` à la première image. Échap (testé **avant** `lost_focus()`) annule ; sinon `lost_focus()` valide si le texte a changé ;
Tab valide puis ouvre le champ suivant de `order`. Fermer l'éditeur si le widget n'est plus dans la zone visible.

`forms_choice_popup` : `Area` + `Frame::popup(ui.style())` (`EG containers/frame.rs:214`) + `ScrollArea` ; une ligne `selectable_label` par option (`label`) ; zone de liste à choix multiple : `ui.checkbox` par option ; `commit_form(nom, Choice(..))` à chaque changement ;
clic hors fenêtre ou Échap ferme.

`commit_form(name, value)` : `self.forms.keep_texture = true; self.worker.send(Request::Edit(EditOp::FillForm { values: vec![(name, value)] }));` + message d'état `status.form_updated`.

### 5.6 Panneau « Champs du formulaire » (`forms_panel`)

`egui::Panel::right("forms")` (comme `comments`, `app.rs:1815` ; `EG containers/panel.rs:256`) affiché si `document.is_some() && forms.panel_open && !organize.active`. Contenu :

1. En-tête : titre `forms.panel.title`, compte `forms.panel.count`, bouton fermer (`tools.close` existant) → `execute(ToggleForms)`.
2. Bandeaux (`forms.banner.xfa`, `forms.banner.scripts`, `forms.banner.encrypted`) selon `info.kind`, `info.has_scripts`, `info.protected`.
3. Bascule « Cette page / Tout le document » (`forms.panel.scope_page|scope_all`), case `forms.highlight` (surbrillance hors outil : `forms.overlay`).
4. `ScrollArea` : champs groupés par page (`forms.panel.page`), triés par `order`. Une ligne par champ : `ui.label(tooltip ou nom)` + contrôle, accessible par `.labelled_by(label.id)` (`EG response.rs:1003`) :

| Genre | Contrôle | Validation |
|---|---|---|
| Texte | `TextEdit::singleline` / `multiline` (3 lignes) sur `forms.drafts[nom]` (défaut : valeur du modèle), `char_limit` | Entrée (une ligne) ou perte de focus avec valeur modifiée ; Échap restaure |
| Case | `ui.checkbox(&mut b, libellé)` | au clic |
| Radio | un `ui.radio_value` par `options` | au clic |
| Liste déroulante | `egui::ComboBox::from_id_salt(("form-combo", nom))` (motif de `tools.rs::position_combo`) | au choix |
| Zone de liste | une `ui.checkbox` par option (multiple) ou `radio_value` (simple) | au clic |
| Bouton / signature | libellé grisé `forms.panel.unsupported_button|signature` | – |

Les contrôles sont `add_enabled(!read_only && !info.protected && info.kind == AcroForm)`. Un clic sur le nom du champ : `go_to_page(page du premier widget)` + `forms.focus` (anneau de focus sur la page).
Pour éviter l'emprunt croisé : `let info = self.forms.info.take(); … ; self.forms.info = info;` ; les validations sont accumulées dans `Vec<(String, FieldValue)>` et envoyées en **une** `EditOp::FillForm` après le dessin.
Au-delà de 200 champs, ne lister par défaut que la page courante (déjà le défaut de `scope_all = false`).

### 5.7 `DocumentChanged` sans clignotement

`app.rs:422-461` efface la texture à chaque modification. Pour une saisie de champ, une page blanche + spinner à chaque validation est inacceptable : remplacer
`self.texture = None;` par

```rust
if !std::mem::take(&mut self.forms.keep_texture) { self.texture = None; }   // l'ancienne image reste affichée jusqu'à l'arrivée du nouveau rendu
```

(`last_rendered = None`, `in_flight = None` et `generation += 1` sont conservés : un nouveau rendu est demandé immédiatement).

### 5.8 Bandeau d'information et boîte « Aplatir »

* Bandeau : `forms_bar(ctx, ui)` appelé dans le `CentralPanel` du document juste après `error_banner(ui)` (`app.rs:~1840`), visible si `doc.info.form == FormKind::AcroForm && !forms.bar_dismissed && tool != FillForm` : `Frame` bleu clair, texte `forms.bar.text`, bouton `menu.forms.fill_tool`, bouton `forms.bar.dismiss`.
* Boîte (`flatten_dialog(&mut self, ctx)`, appelée avec les autres dialogues, `app.rs:1877`) : **obligatoirement** `dialog::show` (D020, `DESK/dialog.rs`) :

```rust
let spec = dialog::Spec::new("flatten", tr(lang, "dialog.flatten.title"), self.settings.dark_theme, self.has_icons)
    .subtitle(tr(lang, "dialog.flatten.subtitle")).icon(glyph::COMBINE, "⧉").width(480.0);
dialog::show(ctx, spec,
    |ui| { ui.label(tr(lang, "dialog.flatten.text"));
           ui.horizontal(|ui| { ui.radio_value(&mut self.forms.flatten_all_pages, true, tr(lang, "stamp.scope.all"));
                                ui.radio_value(&mut self.forms.flatten_all_pages, false, tr(lang, "stamp.scope.current")); }); },
    |ui| { if dialog::primary(ui, tr(lang, "dialog.flatten.ok")) || ui.input(|i| i.key_pressed(Key::Enter)) { submit = true; }
           if dialog::secondary(ui, tr(lang, "dialog.password.cancel")) || ui.input(|i| i.key_pressed(Key::Escape)) { cancel = true; } });
```

  Validation : `EditOp::FlattenForm { pages: if all { None } else { Some(vec![page]) } }`. Ajouter `|| self.forms.flatten_open` à la liste des modales de `handle_input` (`app.rs:~592-596`, à côté de `self.tools.split_open`).
* Export/import : `rfd::FileDialog` (déjà utilisé, motif `tools.rs::save_dialog`) ; export → `BatchOp::ExportFormData` via `run_batch(op, "tools.running.form_export")` ; import → lecture du fichier, `serde_json::from_str`, `plan_from_json`, `EditOp::FillForm`, message `status.form_imported` ; JSON invalide → `error = Some("error.form_import_invalid")`.

### 5.9 Outil `Tool::FillForm` (`DESK/annotate.rs`)

* `Tool` (l. 10-23) : `FillForm` ; `Tool::ALL: [Tool; 12]` ; `command()` → `Command::ToolFillForm` ; `markup()` inchangé (`None`).
* `finish_drag` (l. 190) : arme `Tool::FillForm => {}` (un glissement ne sélectionne pas de texte) ; `paint_selection` : ne pas dessiner le rectangle de glissement pour `FillForm`.
* `tool_palette` (l. 108-150) : séparateur avant `Tool::FillForm` (`if tool == Tool::SelectObject || tool == Tool::FillForm`).
* `execute` : ajouter `Command::ToolFillForm` à la liste des outils (l. 923-941) ; à l'activation : `forms.panel_open = true`, `forms.stale = true`. Les autres commandes : `ToggleForms` bascule `panel_open` ;
  `ClearForm` → `EditOp::ClearForm` ; `FlattenForm` → `forms.flatten_open = true` ; `ExportFormData` / `ImportFormData` → §5.8.
* `comments_panel` (l. 303) : **ne pas lister** les annotations de type `Widget` (elles ont leur panneau ; un bouton « supprimer » détruirait un champ) : `if a.kind == "Widget" { continue; }` (la clé `comments.kind.widget` existe déjà, l. `annotation_label`).

### 5.10 Accessibilité (audit à documenter dans `docs/ACCESSIBILITY.md`)

| Contrôle | Nom accessible (clé) | Clavier |
|---|---|---|
| Rectangle de champ sur la page | `a11y.forms.widget` (`Champ {name}, type {kind}`) ou `a11y.forms.widget_value` | Tab / Maj+Tab (focus natif egui, ordre de création = ordre de lecture), Entrée / Espace = clic |
| Éditeur en place | `a11y.forms.editor` (`Saisie du champ {name}`) | Entrée valide, Échap annule, Tab valide et passe au suivant |
| Fenêtre de choix | `a11y.forms.option` (`{name} : {option}`) | flèches (egui), Entrée, Échap |
| Case du panneau | libellé du champ (`ui.checkbox`) + état `a11y.forms.checked/unchecked` | Espace |
| Zone de texte du panneau | `labelled_by` le libellé du champ | Tab, Entrée, Échap |
| Panneau | `a11y.forms.panel` (info-bulle du bouton du rail, titre du panneau) | Ctrl+Maj+F |
| Bandeaux, boutons, boîte « Aplatir » | textes traduits ; `dialog::show` (Entrée/Échap) | Tab |

Contraste : les teintes de surbrillance reprennent `theme::BLUE` / `ERROR` à 12 % de remplissage et 100 % de contour (contour 3:1 minimum à vérifier par la capture en thème sombre). **Vérification manuelle Narrateur Windows : NON FAITE**, à marquer telle quelle
dans `docs/ACCESSIBILITY.md` tant qu'elle n'a pas été exécutée ; limite connue : la page est une image (texture) dont le texte n'est pas exposé aux lecteurs d'écran, seuls les rectangles de champs et le panneau le sont. Lecture de la structure balisée (Tagged PDF) : hors de cette tranche.

### 5.11 Onglets, annulation, enregistrement

* **Onglets** : `FormsState` fait partie de `TabState` ; la relecture est redemandée à la reprise (`on_resumed`). Le document inactif est suspendu comme octets : rien de spécifique.
* **Annulation** : chaque `FillForm`, `ClearForm`, `FlattenForm`, `AddFormField` est un niveau d'historique (instantané complet, `State::push_undo`, plafonds 50 niveaux / 512 Mo). Une saisie = un niveau (limite assumée : un formulaire de 40 champs = 40 niveaux ; un « mode lot » à bouton Appliquer est une évolution possible).
* **Enregistrer** : `Request::Save` inchangé (`save_as` atomique) ; les apparences sont déjà mémorisées (`[V6]`).

### 5.12 Coordination avec les autres tranches (`docs/plans/`)

| Tranche | Interaction | Action |
|---|---|---|
| `tool-hub.md` | **Supprime `ToolTab` et `tool_row` et la palette flottante** ; table `workspace::TABLE` ; `Fill & Sign` et `Prepare a form` ont `entry: None` (contrat §5.9 : ajouter les commandes au registre, passer `entry` à `Some(..)`, ajouter `HubXxx` si mode) | Si `tool-hub` atterrit **avant** : remplacer §5.2 par : `Workspace::FillSign` passe en `Entry::Mode(HubFillSign)` avec `commands = [ToolFillForm, ToggleForms, ClearForm, FlattenForm, ExportFormData, ImportFormData]` (+ commandes de `sign-image` si ses auteurs les y rattachent) ; `Workspace::PrepareForm` : `Mode(HubPrepareForm)` avec `[ToolAddTextField, ToolAddCheckbox]` (étape 11 seulement : sinon `entry: None`). `Tool::FillForm` n'a plus de palette, il vit dans la barre contextuelle. `Menu::Forms` reste (chaque commande a un `Menu`). Couleur : celle de `TABLE` (`65A30D`). Ne pas ajouter `ToolTab::Forms` |
| `annotations-advanced.md` | `ensure_appearances`, `load_for_edit`, `annots_mut`, `PageGeometry`, `lopdf_err` pub(crate), mêmes gardes de chiffrement ; raccourcis `A`, `D`, `F` ; ajoute des `Tool` | Une seule définition de chaque utilitaire (le premier atterri) ; `Tool::ALL` : ajout additif ; ne pas réutiliser la touche `F` |
| `continuous-view.md` | Réécrit `document_area`, plusieurs pages visibles, F5 | `forms_overlay` appelée par page ; ne pas utiliser F5 |
| `sign-image.md` | Même zone « Fill & Sign », touche `S`, `Request::ObjectInfo` | Aucune collision de commandes ; avertissement de signature numérique commun (question ouverte Q4) |
| `export-convert.md`, `bookmarks-links.md` | Ajoutent des variantes d'`EngineError`, de `EditOp`, des clés i18n | Ajouts additifs ; l'orchestrateur résout les conflits des deux catalogues JSON |

---

## 6. CLI (`apps/sunipdf-cli/src/main.rs`)

Sortie JSON sur stdout, messages d'erreur JSON sur stderr (`{ "error": code, "message": … }`), codes de sortie inchangés (1 pour les erreurs de formulaire, 3 mot de passe, 4 format, 5 E/S).

```rust
/// List form fields (name, type, value, pages, rectangles, options) as JSON.
FormFields {
    file: PathBuf,
    /// Print only the {name: value} map (same format accepted by `fill-form --values`).
    #[arg(long)] values_only: bool,
    #[arg(long)] password: Option<String>,
},
/// Fill fields from a JSON object (--values FILE, or - for stdin) and/or --set NAME=VALUE (repeatable); optionally flatten.
FillForm {
    file: PathBuf,
    #[arg(long)] out: PathBuf,
    #[arg(long)] values: Option<PathBuf>,
    #[arg(long = "set", value_name = "NAME=VALUE")] set: Vec<String>,
    /// Ignore unknown or invalid fields instead of failing (reported in the output).
    #[arg(long)] ignore_unknown: bool,
    #[arg(long)] flatten: bool,
    #[arg(long)] password: Option<String>,
},
/// Flatten form fields and annotations into the page content (pages: "all" or "1,3-5").
Flatten {
    file: PathBuf,
    #[arg(long, default_value = "all")] pages: String,
    #[arg(long)] out: PathBuf,
    #[arg(long)] password: Option<String>,
},
```

Les noms de sous-commandes `form-fields`, `fill-form`, `flatten` viennent du kebab-case automatique de clap (comme `page-numbers`).

`form-fields` :

```json
{
  "kind": "acroform",            // "none" | "acroform" | "xfa"
  "scripts": false, "protected": false, "count": 13,
  "fields": [
    { "name": "paiement", "type": "radio", "value": "virement", "read_only": false, "required": false, "tooltip": null,
      "options": [ { "value": "carte", "label": "carte" }, { "value": "virement", "label": "virement" }, { "value": "especes", "label": "especes" } ],
      "widgets": [ { "page": 1, "rect": [200, 496, 214, 510], "on_state": "carte" }, … ] },
    { "name": "ville", "type": "text", "value": "Élodie Müller", "multiline": false, "max_length": null, … },
    { "name": "adresse.code", "type": "text", "value": "", "max_length": 5, … },
    { "name": "pays", "type": "combo", "value": "BE", "options": [ { "value": "FR", "label": "France" }, … ], … },
    { "name": "langues", "type": "list", "multi_select": true, "value": ["Anglais"], … },
    { "name": "accepte", "type": "checkbox", "value": false, … }
  ]
}
```

Pages en base 1, `rect` = `/Rect` en espace utilisateur (comme `search`). `type` ∈ `text, checkbox, radio, combo, list, button, signature` ; `value` : voir §3.10 (`null` pour boutons/signatures et choix vide).
`fill-form` : valeurs lues par `plan_from_json` + `--set` (`FormField::coerce_text`) ; **mode strict par défaut** : tout nom inconnu ou valeur refusée = erreur (`form_field_not_found`, `form_value_invalid`), aucune écriture ;
`--ignore-unknown` : mode tolérant ; sortie :

```json
{ "out": "remplis.pdf", "applied": 5, "skipped": { "unknown": ["x"], "invalid": [ { "field": "ref", "reason": "read-only" } ] }, "flattened": false, "page_count": 1 }
```

`flatten` : `{ "out": …, "annotations": 17, "pages": 1 }`. Un document XFA : erreur `form_xfa` ; chiffré (écriture) : `security_restricted`.
Écriture par `doc.save_as(&out)` (atomique, comme `rotate`). Test unitaire de `--set` (`name=valeur=avec=égal` → coupe au premier `=`) dans `main.rs` (`mod tests`, l. ~670) ; test d'intégration §8.3.

---

## 7. i18n : clés à ajouter (fr-FR et en-US, mêmes jeux de clés, mêmes `{param}`)

Ajouter aux deux fichiers `assets/i18n/fr-FR.json` / `en-US.json` dans la même livraison ; `scripts\check-i18n.ps1` et les tests de `sunipdf-i18n` vérifient parité, valeurs non vides et paramètres identiques.
Les clés `shortcut.*` utilisent le même libellé dans les deux langues sauf mention.

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.forms` | Formulaires | Forms |
| `menu.forms.fill_tool` | Remplir le formulaire | Fill form |
| `menu.forms.panel` | Panneau des champs du formulaire | Form fields panel |
| `menu.forms.clear` | Effacer les champs | Clear fields |
| `menu.forms.flatten` | Aplatir le formulaire et les annotations… | Flatten form and annotations… |
| `menu.forms.export` | Exporter les valeurs du formulaire (JSON)… | Export form values (JSON)… |
| `menu.forms.import` | Importer des valeurs dans le formulaire (JSON)… | Import values into the form (JSON)… |
| `shortcut.forms_panel` | Ctrl+Maj+F | Ctrl+Shift+F |
| `toolbar.forms` | Formulaires | Forms |
| `forms.bar.text` | Ce document contient un formulaire à remplir. | This document contains a form to fill in. |
| `forms.bar.dismiss` | Masquer | Dismiss |
| `forms.panel.title` | Champs du formulaire | Form fields |
| `forms.panel.count` | {count} champ(s) | {count} field(s) |
| `forms.panel.scope_page` | Cette page | This page |
| `forms.panel.scope_all` | Tout le document | Whole document |
| `forms.panel.page` | Page {page} | Page {page} |
| `forms.panel.empty` | Aucun champ de formulaire sur cette page. | No form fields on this page. |
| `forms.panel.empty_doc` | Ce document ne contient aucun champ de formulaire. | This document contains no form fields. |
| `forms.panel.no_widget` | Champ sans emplacement visible | Field with no visible location |
| `forms.panel.read_only` | Lecture seule | Read-only |
| `forms.panel.required` | Obligatoire | Required |
| `forms.panel.choice_none` | (aucun) | (none) |
| `forms.panel.unsupported_signature` | Champ de signature : non pris en charge dans cette version. | Signature field: not supported in this version. |
| `forms.panel.unsupported_button` | Bouton : sans action dans SuniPDF. | Button: has no action in SuniPDF. |
| `forms.hint` | Cliquez un champ surligné de la page ou utilisez ce panneau. Tab passe au champ suivant, Échap annule la saisie. | Click a highlighted field on the page or use this panel. Tab moves to the next field, Esc cancels typing. |
| `forms.highlight` | Surligner les champs | Highlight fields |
| `forms.kind.text` | Texte | Text |
| `forms.kind.checkbox` | Case à cocher | Checkbox |
| `forms.kind.radio` | Boutons radio | Radio buttons |
| `forms.kind.combo` | Liste déroulante | Drop-down list |
| `forms.kind.list` | Zone de liste | List box |
| `forms.kind.button` | Bouton | Button |
| `forms.kind.signature` | Signature | Signature |
| `forms.banner.xfa` | Formulaire XFA : SuniPDF lit les champs AcroForm mais ne peut ni les remplir ni les aplatir. | XFA form: SuniPDF can read AcroForm fields but cannot fill or flatten them. |
| `forms.banner.scripts` | Ce formulaire contient des scripts (calculs, formats). SuniPDF ne les exécute pas : les champs calculés ne sont pas mis à jour. | This form contains scripts (calculations, formats). SuniPDF does not run them: calculated fields are not updated. |
| `forms.banner.encrypted` | Document protégé : retirez la protection (menu Sécurité) pour remplir le formulaire. | Protected document: remove the protection (Security menu) to fill in the form. |
| `forms.filter_json` | Valeurs de formulaire (JSON) | Form values (JSON) |
| `forms.pick_import` | Choisir le fichier de valeurs à importer | Choose the values file to import |
| `forms.save_export` | Enregistrer les valeurs du formulaire | Save form values |
| `dialog.flatten.title` | Aplatir le formulaire | Flatten form |
| `dialog.flatten.subtitle` | Intégrer champs et annotations au contenu des pages | Merge fields and annotations into the page content |
| `dialog.flatten.text` | Les valeurs saisies, les surlignages et les notes deviennent du contenu fixe : ils ne seront plus modifiables. Vous pouvez annuler avec Ctrl+Z tant que le document n'est pas fermé. | Entered values, highlights and notes become fixed content and can no longer be edited. You can undo with Ctrl+Z until the document is closed. |
| `dialog.flatten.ok` | Aplatir | Flatten |
| `status.form_updated` | Champ « {name} » mis à jour | Field “{name}” updated |
| `status.form_cleared` | {count} champ(s) effacé(s) | {count} field(s) cleared |
| `status.form_flattened` | Formulaire aplati | Form flattened |
| `status.form_imported` | {applied} valeur(s) importée(s), {skipped} ignorée(s) | {applied} value(s) imported, {skipped} skipped |
| `status.form_unsupported_widget` | Ce champ ne peut pas être modifié ici. | This field cannot be edited here. |
| `tools.running.form_export` | Export des valeurs… | Exporting values… |
| `tools.done.form_export` | Valeurs exportées dans {name} ({count} champs) | Values exported to {name} ({count} fields) |
| `error.form_field_not_found` | Ce champ n'existe pas dans le formulaire. | This field does not exist in the form. |
| `error.form_value_invalid` | Valeur refusée : champ en lecture seule, longueur dépassée, choix inexistant ou nom de champ invalide. | Value rejected: read-only field, length exceeded, unknown choice or invalid field name. |
| `error.form_xfa` | Les formulaires XFA ne sont pas pris en charge. | XFA forms are not supported. |
| `error.form_import_invalid` | Fichier de valeurs invalide : un objet JSON « nom du champ : valeur » est attendu. | Invalid values file: a JSON object “field name: value” is expected. |
| `a11y.forms.panel` | Panneau des champs du formulaire | Form fields panel |
| `a11y.forms.widget` | Champ {name}, type {kind} | Field {name}, type {kind} |
| `a11y.forms.widget_value` | Champ {name}, type {kind}, valeur : {value} | Field {name}, type {kind}, value: {value} |
| `a11y.forms.checked` | coché | checked |
| `a11y.forms.unchecked` | non coché | unchecked |
| `a11y.forms.option` | {name} : {option} | {name}: {option} |
| `a11y.forms.editor` | Saisie du champ {name} | Editing field {name} |

Clés **optionnelles** (étape 11) : `menu.forms.add_text` (Ajouter un champ texte / Add a text field), `menu.forms.add_checkbox` (Ajouter une case à cocher / Add a checkbox), `dialog.field.title` (Nouveau champ de formulaire / New form field),
`dialog.field.subtitle` (Le nom identifie la valeur dans les exports / The name identifies the value in exports), `dialog.field.name` (Nom du champ / Field name), `dialog.field.multiline` (Plusieurs lignes / Multiple lines),
`dialog.field.required` (Champ obligatoire / Required field), `dialog.field.ok` (Ajouter / Add). Réutilisés (déjà présents) : `stamp.scope.all`, `stamp.scope.current`, `dialog.password.cancel`, `tools.close`, `error.security`, `comments.kind.widget`.

Tests i18n côté bureau : ajouter `include_str!("forms.rs")` à la concaténation de `ui_keys_used_by_app_exist` (`app.rs:~1960`, vérifie que toute chaîne en forme de clé existe) en veillant à ce qu'aucune chaîne de `forms.rs` n'ait la forme d'une clé sans l'être
(les identifiants `Id::new("form-widget")` contiennent un tiret, donc sont ignorés) ; le test `registry_keys_exist_in_both_catalogs` couvre les libellés de commandes.

---

## 8. Tests

### 8.1 Moteur : `crates/sunipdf-core/tests/engine.rs` (intégration, même `engine()` sérialisé que les tests existants)

Fixtures : `acroform.pdf`, `acroform-noap.pdf`, `acroform-xfa.pdf` (annexe A). Aides à ajouter en tête du fichier de test :

```rust
fn rect(l: f32, b: f32, r: f32, t: f32) -> PointRect { PointRect { left: l, bottom: b, right: r, top: t } }
/// Pixels sombres (RVB < 100) dans un rectangle PDF ; rendu à l'échelle 1,0 (1 px = 1 pt).
fn dark(img: &sunipdf_core::RenderedPage, r: PointRect, page_h: f32) -> usize { /* parcours y de page_h-top à page_h-bottom, x de left à right */ }
/// Texte de l'apparence /AP /N du widget d'un champ, relu avec lopdf (dépendance du crate, utilisable par les tests d'intégration).
fn ap_text(bytes: &[u8], field: &str) -> String { /* lopdf::Document::load_mem ; objets dont /T décodé == field et /Subtype /Widget ;
    get_dict_in_dict(d, b"AP") -> /N Reference -> Stream::get_plain_content (LP object.rs:834) -> from_utf8_lossy */ }
```

Ordres de grandeur mesurés (PDFium 7881, échelle 1,0, `acroform.pdf`) : `nom` 0, `ville` 173, `ref` 98, case 0, radio *carte* 0, radio *virement* 64 ; `acroform-noap.pdf` : `nom` 556 (bordure), `ville` 729. Assertions **relatives** (marges larges).

| # | Test | Assertions |
|---|---|---|
| T1 | `form_lists_fields_types_values_and_widgets` | `form_kind() == AcroForm` ; `doc.info().form == AcroForm` ; 13 champs de noms attendus (`nom, ville, commentaire, ref, accepte, paiement, pays, langues, adresse.rue, adresse.code, telephone, signature, envoyer`) ; genres ; `ville` = `Text("Élodie Müller")` (accents PDFDoc) ; `ref` `read_only` ; `commentaire` `multiline` ; `nom.tooltip == Some("Nom et prenom")` ; `adresse.code.max_len == Some(5)` ; `paiement` : 3 widgets, options `carte, virement, especes`, valeur `Choice(["virement"])`, `no_toggle_off` ; `telephone` : 2 widgets ; `pays` : `Choice(["BE"])`, options `("FR","France") …` ; `langues` : `multi_select`, `Choice(["Anglais"])` ; `accepte` : `Bool(false)`, `on_state == Some("Yes")` ; `signature` genre `Signature`, valeur `None` ; `has_scripts == false`, `protected == false` ; `widgets[0].rect` de `nom` = (150, 696, 410, 716) et `display == rect` (page non pivotée) ; document sans AcroForm (`two-pages.pdf`) → `FormInfo::default()` |
| T2 | `form_fill_roundtrip_values_and_pixels` | page_h = 792 ; pixels avant : `nom` 0, case 0, radio *carte* 0, *virement* ≥ 30. `fill_form(Strict)` de : `nom="Jean Dupont"`, `ville="Zoë"`, `commentaire="ligne 1\nligne 2"`, `accepte=true`, `paiement=["carte"]`, `pays=["Canada"]` (libellé accepté), `langues=["Français","Allemand"]`, `telephone="0102030405"`, `adresse.rue="1 rue X"`. `report.applied.len() == 9`. Relecture `form_info()` : valeurs égales (`pays` = `["CA"]`, `langues` = `["Français","Allemand"]`, `telephone` identique pour les 2 widgets). Pixels après : `nom` > 300, case > 10, *carte* > 30, *virement* == 0. **Aller-retour** : `snapshot()` → `engine.open_bytes` → mêmes valeurs et mêmes rendus ; `save_as` dans un fichier temporaire → `open` → idem |
| T3 | `form_fill_errors_are_typed_and_atomic` | `Strict` : nom inconnu → `FormFieldNotFound("zzz")` ; `ref` → `FormValueInvalid` (lecture seule) ; `nom = Bool(true)` → `FormValueInvalid` ; `pays = ["XX"]` invalide, `["FR","BE"]` invalide ; `adresse.code = "123456"` invalide, `"75001"` valide ; `signature`, `envoyer` invalides ; `paiement = []` invalide (`no_toggle_off`) ; `commentaire` multiligne accepte `\n`, `nom` (une ligne) refuse `\n`. **Atomicité** : `[("nom","A"),("zzz","B")]` en `Strict` → `Err` et `nom` reste vide ; en `Lenient` → `applied == ["nom"]`, `skipped_unknown == ["zzz"]` |
| T4 | `form_appearance_is_materialized_in_saved_bytes` | `acroform-noap.pdf` : `fill_form(nom = "Jean Dupont")` ; `ap_text(&doc.snapshot(), "nom")` contient `(Jean Dupont) Tj` **(preuve que l'apparence est dans le fichier, pas seulement régénérée au chargement `[V6]`)** ; idem `ville` avec `(\xC9lodie M\xFCller) Tj` sur `acroform.pdf` après `fill_form(ville = "Élodie Müller")` (AP supprimé puis régénéré) ; liste déroulante : `ap_text("pays")` contient `(Canada)` après `["CA"]` |
| T5 | `flatten_keeps_content_and_removes_widgets` | Sur `acroform.pdf` : `fill_form(nom = "Jean Dupont")` ; `add_markup(Highlight)` sur une sélection de texte (annotation `/F 0` créée par PDFium `[V8]`) ; pixels : `ville` 173, jaune > 500. `flatten(None)` → `Ok(n)` avec `n ≥ 17` ; `annotations(0)` vide ; `form_kind() == None` ; `info().form == None` ; `form_info().fields.is_empty()` ; **rendu** : `ville` (widget `/F 0`) > 100 px, `nom` > 300, jaune > 500 (non-régression du piège `FLAT_PRINT`). `save_as` → rouvrir → mêmes comptes. `flatten(None)` une seconde fois → `Ok(0)`. `acroform-noap.pdf` : `flatten` sans rendu préalable conserve `ville` (> 300 px) |
| T6 | `form_xfa_is_reported_and_blocks_write` | `acroform-xfa.pdf` : `form_kind() == Xfa` ; `form_info().kind == Xfa` et 13 champs lisibles ; `fill_form` → `Err(FormXfa)` ; `flatten(None)` → `Err(FormXfa)` ; `fill_form` code stable `form_xfa` |
| T7 | `form_write_refuses_encrypted_document` | `encrypted-rc4.pdf` ouvert avec `secret` : `fill_form(&[("x".into(), Text("y".into()))], Strict)` → `Err(SecurityRestricted)` ; `flatten(None)` ne renvoie pas d'erreur de chiffrement ; l'instantané reste chiffré (`properties().encrypted`) |
| T8 | `form_display_rect_follows_page_rotation` | `set_page_rotation(0, R90)` : pour `nom`, `rect` inchangé (150, 696, 410, 716) et `display` ≈ (696, 202, 716, 462) (±0,25) ; largeur ≈ 20 et hauteur ≈ 260 ; contenu dans la page affichée 792×612. Idem R180 et R270 (valeurs du constat V12) |
| T9 | `form_json_export_import_roundtrip` | `values_to_json(info)` : `ville == "Élodie Müller"`, `accepte == false`, `paiement == "virement"`, `pays == "BE"`, `langues == ["Anglais"]`, aucune clé `signature`/`envoyer`. `plan_from_json` sur `{"nom":"Zoé","accepte":"oui","paiement":"carte","pays":"Canada","langues":["Allemand"],"inconnu":1,"ref":"x","adresse.code":"123456"}` : 5 valeurs, `skipped_unknown == ["inconnu"]`, `skipped_invalid` contient `ref` et `adresse.code` ; `fill_form(plan.values, Strict)` réussit ; ré-export = attendu. Non-objet JSON → `FormValueInvalid` |
| T10 | `form_clear_resets_editable_fields` | Après T2 : `clear_form()` ; `nom == Text("")`, `accepte == Bool(false)`, `pays == Choice([])`, `langues == Choice([])` ; `ref` toujours `REF-42` ; `paiement` inchangé (radio `no_toggle_off`) ; compte renvoyé = champs modifiés |
| T11 | `lopdf_model_matches_pdfium_enumeration` (**test unitaire dans `forms.rs`**, `#[cfg(test)]`) | Oracle : `Engine::new()` (un seul test du crate l'utilise : pas de verrou nécessaire, le binaire de tests unitaires est distinct de `tests/engine.rs`), ouvrir `acroform.pdf`, énumérer `page.annotations().iter()` → `annotation.as_form_field()` (`PR/.../annotation.rs:832`) → `name()` et `field_type()` (`PR/.../field.rs:288-300`) ; comparer, triés, à `(nom, genre)` de chaque widget du modèle lopdf : mêmes 16 widgets |
| T12 (optionnel, étape 11) | `form_create_text_and_checkbox` | Page vierge (`two-pages.pdf` n'a pas d'AcroForm) : `add_form_field(Text, "champ1")`, `add_form_field(Checkbox, "case1")` → `form_kind() == AcroForm`, `form_info()` les liste (non homonyme, nom avec `.` refusé, rectangle < 4 pt refusé) ; remplissage (`champ1="Bonjour"`, `case1=true`) puis rendu : pixels > seuil ; `ap_text("champ1")` contient `(Bonjour) Tj` ; aplatissement réussi |

### 8.2 Interface (`cargo test -p sunipdf-i18n -p sunipdf-desktop -j 2`)

* `forms.rs` : `reading_order_sorts_by_page_then_top_then_left` (`FormInfo` synthétique, sans PDFium), `draft_to_value_per_kind`, `kind_key_exists_in_both_catalogs` (chaque `forms.kind.*`), `forms_keys_exist` (§7).
* `app.rs::error_key` : tableau du test étendu aux trois variantes ; `registry_keys_exist_in_both_catalogs` couvre les 6 libellés.
* `command.rs` : tests existants inchangés (7 commandes dans `Menu::File`).

### 8.3 CLI (`apps/sunipdf-cli/tests/forms_cli.rs`, nouveau)

Exécute le binaire (`env!("CARGO_BIN_EXE_sunipdf-cli")`) sur `../../crates/sunipdf-core/tests/fixtures/acroform.pdf` : `form-fields` → JSON : 13 champs, `kind == "acroform"`, pages en base 1 ;
`form-fields --values-only` ; `fill-form --set nom=Jean --set accepte=true --out <tmp>` puis `form-fields <tmp>` : valeurs relues ; `fill-form --set inconnu=1` → code de sortie 1 et `"error":"form_field_not_found"` sur stderr,
**aucun fichier produit** ; avec `--ignore-unknown` → succès et `skipped.unknown == ["inconnu"]` ; `fill-form … --flatten` puis `form-fields` → `count == 0` ; `flatten` ; `acroform-xfa.pdf` + `fill-form` → `"error":"form_xfa"`.

### 8.4 Scénarios de capture manuelle (`scripts/dev-screenshot.ps1`)

Prérequis : `cargo build -p sunipdf-desktop -j 2`. Paramètres du script : `-Pdf`, `-Commands 'id1,id2'` (exécutés après l'ouverture, `SUNIPDF_STARTUP_COMMANDS`), `-Drag 'x1,y1,x2,y2;…'` (un **clic** = glissement de longueur nulle `x,y,x,y`), `-KeysAfter` (syntaxe `SendKeys` : `{ENTER}`, `{TAB}`).

```powershell
$f = "crates\sunipdf-core\tests\fixtures\acroform.pdf"
# S1 : surbrillance + panneau (vérifier : 16 rectangles, `ref` en gris, panneau à 13 lignes, bandeau absent car l'outil est actif)
.\scripts\dev-screenshot.ps1 -Out forms-1.png -Pdf $f -Commands "forms.fill"
# S2 : bandeau d'information à l'ouverture (sans commande)
.\scripts\dev-screenshot.ps1 -Out forms-2.png -Pdf $f
# S3 : clic sur la case « Accepte les conditions » (coordonnées à calibrer sur S1, voir ci-dessous)
.\scripts\dev-screenshot.ps1 -Out forms-3.png -Pdf $f -Commands "forms.fill" -Drag "X,Y,X,Y"
# S4 : saisie en place dans « Nom » : clic, texte, Entrée (le texte apparaît dans le champ avec bordure régénérée)
.\scripts\dev-screenshot.ps1 -Out forms-4.png -Pdf $f -Commands "forms.fill" -Drag "X2,Y2,X2,Y2" -KeysAfter "Marie{ENTER}"
# S5 : boîte « Aplatir » puis validation par Entrée : plus de surbrillance, panneau vide, valeurs dans la page
.\scripts\dev-screenshot.ps1 -Out forms-5.png -Pdf $f -Commands "forms.fill,forms.flatten" -KeysAfter "{ENTER}"
# S6 : thème sombre et anglais : repasser S1 après avoir basculé le thème/la langue dans les paramètres (contraste du contour)
# S7 : formulaire XFA : bandeau « XFA » et contrôles grisés
.\scripts\dev-screenshot.ps1 -Out forms-7.png -Pdf "crates\sunipdf-core\tests\fixtures\acroform-xfa.pdf" -Commands "forms.fill"
```

Calibrage de `X,Y` : sur S1, relever le coin haut-gauche de la page (`page_left`, `page_top` en pixels fenêtre) et le zoom de la barre d'état ; pixel = `page_left + x_pt * k`, `page_top + (792 - y_pt) * k` avec `k = zoom * 96 / 72` ;
case (200..214, 526..540 pt) → viser `(207, 533)`, champ `nom` → `(280, 706)`. Contrôles attendus : chaque capture est ouverte et lue (non seulement produite).

---

## 9. Nouvelles dépendances

| Crate | Version | Licence | Pourquoi |
|---|---|---|---|
| `serde_json` | 1 (déjà dans le workspace : `Cargo.toml` racine, `Cargo.lock` 1.0.154) | MIT OR Apache-2.0 | ajouté à `crates/sunipdf-core/Cargo.toml` (`serde_json = { workspace = true }`) : `values_to_json`, `plan_from_json`, `info_to_json`. Déjà compilé pour la CLI et le bureau : aucun nouveau code tiers dans le binaire |

**Aucune autre dépendance** pour les étapes 1 à 11. `lopdf` (MIT) et `pdfium-render` sont déjà utilisés. Étape 12 (XFDF export) : aucune. (XFDF import, hors périmètre : `quick-xml` 0.41, MIT, déjà dans `Cargo.lock`, non retenu.)
Rien dans `docs/THIRD_PARTY_NOTICES.md` à ajouter (`serde_json` y figure s'il est déjà listé pour la CLI ; sinon l'ajouter avec sa licence).

---

## 10. Risques, pièges connus, ordre d'implémentation, estimation

### 10.1 Pièges (tous vérifiés sauf mention)

1. **`set_value`, `set_checked` de `pdfium-render` : ne pas les utiliser** (`[V2]` à `[V4]`) : silencieux ou faux (`Ok(())` sans effet sur un champ à plusieurs widgets).
2. **Aplatissement** : (a) poser l'indicateur Print (sinon suppression sans dessin) ; (b) `ensure_appearances` avant (sinon un champ sans `/AP` disparaît) ; (c) retirer `/AcroForm` (sinon des champs orphelins) ; (d) ne jamais activer la fonctionnalité Cargo `flatten` (`unimplemented!()`).
3. **`FPDF_RenderPageBitmap` ne dessine pas les champs** ; toute matrice de rendu désactive `FFLDraw` (`render_config.rs:387-391`) : pas de zoom « par matrice » pour les pages de formulaire (la tranche `continuous-view` doit garder `set_target_size`, constat de son §).
4. **Chiffrement** : `security::load` retire `/Encrypt` ; ne jamais écrire un document chiffré par lopdf (garde `SecurityRestricted`). Défaut voisin déjà signalé : `merge_content_streams` (`edit.rs:402`).
5. **Masques de `/Ff`** : ne pas utiliser `PdfFormFieldFlags` pour les bits non fournis par PDFium (décalés d'un bit, `[V14]`).
6. **`impl From<&str> for Object` = Name** (`LP/object.rs:106`) : une valeur de champ texte doit passer par `lopdf::text_string`.
7. **Cycles et profondeur** de `/Kids` (PDF hostile) : `seen` + profondeur 32 ; aucun `unwrap` sur les dictionnaires.
8. **Champs à plusieurs widgets / hiérarchiques** : `V` s'écrit sur le dictionnaire de champ, jamais sur le widget (`[V3]`).
9. **JavaScript absent** : les champs à format/calcul affichent la valeur brute ; avertir, ne pas prétendre l'exécuter.
10. **Polices** : les caractères hors Latin-1 utilisent des polices système non incorporées (`[V7]`) ; l'apparence peut différer ailleurs.
11. **Interface** : `DocumentChanged` efface la texture (`keep_texture`, §5.7) ; `handle_input` retourne tôt si un widget a le focus (d'où `surrender_focus()` après un clic de champ) ; `TextEdit::frame` prend un `Frame` en egui 0.36 (`EG widgets/text_edit/builder.rs:306`), pas un booléen ; `Panel` remplace `SidePanel`.
12. **Coût** : `form_info` = instantané complet + analyse lopdf (≈ taille du fichier) ; limité aux documents qui ont un AcroForm (`DocumentInfo.form`) et à l'outil/panneau ouvert ; chaque validation de champ = 2 sauvegardes PDFium + 1 analyse + 1 rechargement : acceptable < 5 Mo, pénible au-delà (évolution : mode « Appliquer » groupé).
13. **Conflits de fusion** avec les autres tranches (§5.12) : `command.rs`, `app.rs::execute`, `shell.rs`, `worker.rs`, `error.rs`, catalogues, `make_fixtures.py`, `tests/engine.rs`. Modifications additives ; mettre l'essentiel du code dans `forms.rs`, `form_data.rs` (moteur) et `forms.rs` (bureau).
14. **Document signé numériquement** : toute modification invalide la signature ; avertissement transversal (Q4), non traité ici.
15. **Rotation et CropBox** : `display` est correct pour les champs ; les autres surcouches de l'application ne le sont pas (voir `KNOWN_ISSUES` A11 prévu par `annotations-advanced`).

### 10.2 Ordre d'implémentation (chaque étape compile et passe `cargo test --workspace -j 2`, `clippy -D warnings`, `fmt`)

| Étape | Contenu | Effort |
|---|---|---|
| 0 | Fixtures : fonction `acroform()` dans `scripts/make_fixtures.py` + régénération (`python -I scripts\make_fixtures.py crates\sunipdf-core\tests\fixtures`) ; aucun Rust | S |
| 1 | `error.rs` (3 variantes) + `error_key` + test des erreurs + 3 clés `error.form_*` dans les catalogues ; `lopdf_err` en `pub(crate)` | S |
| 2 | `forms.rs` lecture : types, `build_model`, `form_kind`, `form_info`, `display_rect`, `DocumentInfo.form` ; tests T1, T8, T11 | M |
| 3 | Écriture : `validate`, `fill_form`, `ensure_appearances`, `clear_form` ; tests T2, T3, T4, T7, T10 | M |
| 4 | `flatten` + `drop_acroform` ; tests T5, T6 | S à M |
| 5 | `form_data.rs` : JSON, `coerce_text`, `info_to_json` ; `serde_json` dans le crate ; test T9 | S |
| 6 | CLI : `form-fields`, `fill-form`, `flatten` ; test `forms_cli.rs` ; essais manuels | S |
| 7 | Worker : `Request::FormInfo`, `Response::FormInfo`, `EditOp::{FillForm, ClearForm, FlattenForm}`, `BatchOp::ExportFormData` (compile seul) | S |
| 8 | Registre + `Menu::Forms` + commandes + `command_icon`/`command_color` + `ToolTab::Forms` (ou entrée `Workspace` si `tool-hub` a atterri) + clés i18n des menus/commandes + icône/couleur | S |
| 9 | Panneau « Champs du formulaire », bandeau, relecture, `TabState`, `keep_texture`, `forms_panel`, export/import, boîte « Aplatir » ; captures S2, S5, S7 | M |
| 10 | Surcouche de page : `Tool::FillForm`, `forms_overlay`, saisie en place, fenêtre de choix, Tab, accessibilité (`widget_info`), `docs/ACCESSIBILITY.md` ; captures S1, S3, S4, S6 | M à L |
| 11 (optionnelle) | Création : `add_form_field`, `EditOp::AddFormField`, outils `ToolAddTextField` / `ToolAddCheckbox`, boîte « Nouveau champ » ; test T12 | M |
| 12 (optionnelle) | `export_xfdf` (+ commande « Exporter en XFDF » si souhaité) | S |

### 10.3 Estimation globale : **L** (étapes 0 à 10) ; **+M** avec les étapes optionnelles 11 et 12.

---

## 11. Documentation à mettre à jour

| Fichier | Mise à jour |
|---|---|
| `docs/PROJECT_STATUS.md` | Ligne SUNI-0xx « Formulaires AcroForm » (cases cochées seulement avec les commandes réellement exécutées) ; compte de commandes et de tests |
| `docs/NEXT_TASKS.md` | Retirer/mettre à jour la tâche ; reste : XFDF/FDF import, création avancée (radio, liste, signature), mode « Appliquer » groupé, ré-chiffrement après remplissage, Narrateur |
| `docs/FEATURE_MATRIX.md` (section F, l. 83-90) | « Remplir AcroForm, apparences régénérées » → **Fait** (texte, case, radio, listes, aplatissement, JSON, CLI) avec limites ; « Créer des champs » → Fait/Planifié selon l'étape 11 ; « Import/export FDF/XFDF » → JSON fait, XFDF export optionnel, FDF/XFDF import non ; « XFA » → détecté, lecture seule |
| `docs/ROADMAP.md` (l. 34, 37) | Retirer « formulaires » du « Reste » |
| `docs/DECISIONS.md` | **D021** : écriture des champs par lopdf + régénération des apparences par PDFium (motifs `[V2]` à `[V6]`) ; **D022** : aplatissement `FLAT_PRINT` avec indicateur Print et `/AcroForm` retiré ; **D023** : documents chiffrés non remplissables, XFA lecture seule, pas de JS ; **D024** : module `sunipdf-core::forms` plutôt que crate `sunipdf-forms` (si validé Q1) |
| `docs/KNOWN_ISSUES.md` | Nouvelles entrées applicatives : limites du §2.2 ; polices non incorporées ; un niveau d'annulation par champ ; `form_info` coûteux sur gros fichiers ; Narrateur non testé |
| `docs/ARCHITECTURE.md` (tableau « Crates planifiés ») | Ligne `sunipdf-forms` : réalisée comme module `forms` de `sunipdf-core` |
| `docs/I18N.md` | Préfixe `forms.*` désormais utilisé ; `a11y.forms.*` |
| `docs/ACCESSIBILITY.md` | **Créer** : tableau du §5.10, état « Narrateur : non fait », limites (page = image) |
| `docs/USER_GUIDE.fr.md` / `.en.md` | Section « Formulaires » : remplir, panneau, aplatir (avertissement), export/import JSON, limites (XFA, scripts, documents protégés) |
| `CHANGELOG.md` (« Non publié › Ajouté ») | Formulaires AcroForm : liste des fonctions, CLI `form-fields` / `fill-form` / `flatten` |
| `docs/THIRD_PARTY_NOTICES.md` | `serde_json` s'il n'y figure pas déjà |
| `README.md` / aide CLI | Exemples `form-fields`, `fill-form` |

---

## Annexe A : générateur de fixtures (à copier dans `scripts/make_fixtures.py`)

Code **exécuté** pendant la recherche (`python -I`, il produit les trois fichiers ; les constats du §2.3 ont été mesurés dessus). Il réutilise `build` et `stream` du fichier.
À ajouter avant le bloc `if __name__ == "__main__":`, puis dans `main` :

```python
    acroform(os.path.join(out, "acroform.pdf"))
    acroform(os.path.join(out, "acroform-noap.pdf"), with_ap=False)
    acroform(os.path.join(out, "acroform-xfa.pdf"), xfa=True)
```

```python
def acroform(path, with_ap=True, xfa=False):
    """Formulaire AcroForm d'une page (612x792) : texte, multiligne, lecture seule, case à cocher, groupe de
    boutons radio, liste déroulante (export/affichage), liste à choix multiple, champs hiérarchiques,
    champ à deux widgets, signature, bouton. with_ap=False : aucun flux d'apparence pour les champs texte/choix/case
    (PDFium les génère). xfa=True : ajoute /XFA à l'AcroForm."""
    objs = []

    def add(body):
        objs.append(body)
        return len(objs)  # numéro d'objet (1-based)

    def ref(n):
        return b"%d 0 R" % n

    def form_xobject(content, w, h):
        return (b"<< /Type /XObject /Subtype /Form /BBox [0 0 %d %d] /Resources << /Font << /Helv 7 0 R /ZaDb 8 0 R >> >> /Length %d >>\nstream\n"
                % (w, h, len(content))) + content + b"\nendstream"

    def ap_text(w, h, text=b""):
        body = (b"/Tx BMC q BT /Helv 12 Tf 0 g 2 4 Td (%s) Tj ET Q EMC" % text) if text else b"/Tx BMC EMC"
        return form_xobject(body, w, h)

    add(b"")  # 1 catalogue (rempli à la fin)
    add(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>")  # 2
    add(b"")  # 3 page (remplie à la fin)
    labels = [(700, "Nom"), (670, "Ville (accents)"), (640, "Commentaire"), (560, "Reference (lecture seule)"),
              (530, "Accepte les conditions"), (500, "Paiement"), (440, "Pays"), (380, "Langues"),
              (300, "Rue"), (270, "Code postal"), (240, "Telephone"), (200, "Signature")]
    content = b"".join(b"BT /F1 11 Tf 40 %d Td (%s) Tj ET\n" % (y + 4, t.encode("latin-1")) for y, t in labels)
    add(stream(content))  # 4
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")  # 5 police des libellés
    add(b"")  # 6 AcroForm (rempli à la fin)
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")  # 7 /Helv
    add(b"<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>")  # 8 /ZaDb
    fields, annots = [], []

    def new_widget(rect, extra, ap=None, flags=4):
        x1, y1, x2, y2 = rect
        body = b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F %d /Rect [%d %d %d %d] %s" % (flags, x1, y1, x2, y2, extra)
        if ap is not None and with_ap:
            body += b" /AP << /N %s >>" % ap
        n = add(body + b" >>")
        annots.append(n)
        return n

    def apref(w, h, text=b""):
        return ref(add(ap_text(w, h, text))) if with_ap else None

    def apdict(w, h):
        return b"/AP << /N %s >>" % ref(add(ap_text(w, h))) if with_ap else b""

    BOX = b"/MK << /BC [0 0 0] /BG [0.95 0.95 1] >> /DA (/Helv 12 Tf 0 g)"
    # --- champs texte fusionnés avec leur widget ---
    fields.append(new_widget((150, 696, 410, 716), b"/FT /Tx /T (nom) /TU (Nom et prenom) " + BOX, apref(260, 20)))
    # « ville » : widget SANS indicateur d'impression (/F 0) -> piège de l'aplatissement PDFium (mode impression)
    fields.append(new_widget((150, 666, 410, 686), b"/FT /Tx /T (ville) /V (\\311lodie M\\374ller) " + BOX,
                             apref(260, 20, b"\\311lodie M\\374ller"), flags=0))
    fields.append(new_widget((150, 590, 510, 650), b"/FT /Tx /Ff 4096 /T (commentaire) " + BOX, apref(360, 60)))
    fields.append(new_widget((200, 556, 410, 576), b"/FT /Tx /Ff 1 /T (ref) /V (REF-42) " + BOX, apref(210, 20, b"REF-42")))
    # --- case à cocher : AP/N = << /Yes /Off >> ---
    on = add(form_xobject(b"q 0 g 2 2 m 12 12 l 12 2 m 2 12 l S Q", 14, 14))
    off = add(form_xobject(b"", 14, 14))
    fields.append(new_widget((200, 526, 214, 540),
                             b"/FT /Btn /T (accepte) /V /Off /AS /Off /MK << /BC [0 0 0] /CA (8) >> /DA (/ZaDb 0 Tf 0 g)",
                             b"<< /Yes %s /Off %s >>" % (ref(on), ref(off))))
    # --- groupe de boutons radio : champ parent (V = état actif du bouton choisi) + 3 widgets ---
    parent = add(b"")
    kids = []
    for name, y in [("carte", 496), ("virement", 476), ("especes", 456)]:
        a_on = add(form_xobject(b"q 0 g 3 3 8 8 re f Q", 14, 14))
        a_off = add(form_xobject(b"", 14, 14))
        k = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [200 %d 214 %d] /AS %s /MK << /BC [0 0 0] /CA (l) >> /DA (/ZaDb 0 Tf 0 g) /AP << /N << /%s %s /Off %s >> >> >>"
                % (ref(parent), y, y + 14, b"/virement" if name == "virement" else b"/Off", name.encode(), ref(a_on), ref(a_off)))
        kids.append(k)
        annots.append(k)
    objs[parent - 1] = b"<< /FT /Btn /Ff 49152 /T (paiement) /V /virement /Kids [%s] >>" % b" ".join(ref(k) for k in kids)
    fields.append(parent)
    # --- liste déroulante : /Opt en paires [export affichage] ---
    fields.append(new_widget((150, 436, 350, 456),
                             b"/FT /Ch /Ff 131072 /T (pays) /Opt [[(FR) (France)] [(BE) (Belgique)] [(CA) (Canada)]] /V (BE) " + BOX,
                             apref(200, 20, b"Belgique")))
    # --- liste à choix multiple ---
    fields.append(new_widget((150, 350, 350, 410),
                             b"/FT /Ch /Ff 2097152 /T (langues) /Opt [(Fran\\347ais) (Anglais) (Allemand)] /V [(Anglais)] /I [1] " + BOX,
                             apref(200, 60)))
    # --- champs hiérarchiques adresse.rue et adresse.code (MaxLen 5) ---
    addr = add(b"")
    k1 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /FT /Tx /T (rue) /Rect [150 296 410 316] %s %s >>"
             % (ref(addr), BOX, apdict(260, 20)))
    k2 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /FT /Tx /T (code) /MaxLen 5 /Rect [150 266 250 286] %s %s >>"
             % (ref(addr), BOX, apdict(100, 20)))
    objs[addr - 1] = b"<< /T (adresse) /Kids [%s %s] >>" % (ref(k1), ref(k2))
    annots += [k1, k2]
    fields.append(addr)
    # --- un champ à deux widgets (même valeur affichée deux fois) ---
    tel = add(b"")
    w1 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [150 236 300 256] %s %s >>" % (ref(tel), BOX, apdict(150, 20)))
    w2 = add(b"<< /Type /Annot /Subtype /Widget /P 3 0 R /F 4 /Parent %s /Rect [320 236 470 256] %s %s >>" % (ref(tel), BOX, apdict(150, 20)))
    objs[tel - 1] = b"<< /FT /Tx /T (telephone) /Kids [%s %s] >>" % (ref(w1), ref(w2))
    annots += [w1, w2]
    fields.append(tel)
    # --- signature (non remplissable ici) et bouton-poussoir ---
    fields.append(new_widget((150, 190, 350, 230), b"/FT /Sig /T (signature)"))
    fields.append(new_widget((400, 190, 500, 214),
                             b"/FT /Btn /Ff 65536 /T (envoyer) /MK << /BC [0 0 0] /BG [0.8 0.8 0.8] /CA (Envoyer) >> /DA (/Helv 12 Tf 0 g)"))
    acro = b"<< /Fields [%s] /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv 7 0 R /ZaDb 8 0 R >> >>" % b" ".join(ref(f) for f in fields)
    if xfa:
        acro += b" /XFA [(preamble) 4 0 R]"
    objs[5] = acro + b" >>"
    objs[0] = b"<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>"
    objs[2] = (b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> /Annots [%s] >>"
               % b" ".join(ref(a) for a in annots))
    open(path, "wb").write(build(objs))
```

Vérifié sur ces fichiers (PDFium 7881) : 16 widgets, 13 champs, `FPDF_GetFormType` = 1 (3 pour `acroform-xfa.pdf`), `ville` 173 px avant aplatissement et 0 après `FPDFPage_Flatten(FLAT_PRINT)` sans l'indicateur Print.

## Annexe B : questions ouvertes pour l'orchestrateur

* **Q1** : module `sunipdf-core::forms` (ce plan) ou crate `sunipdf-forms` comme dans `docs/ARCHITECTURE.md` ? Le module évite un crate qui dépendrait de `Document` et de ses champs privés.
* **Q2** : documents chiffrés : refuser le remplissage (ce plan, cohérent avec `annotations-advanced.md`) ou ré-appliquer `security::protect` avec les permissions d'origine (travail supplémentaire, risque de changer l'algorithme) ?
* **Q3** : aplatissement : PDFium n'a pas d'aplatissement sélectif ; accepter « formulaire **et** annotations » en une opération (ce plan) ?
* **Q4** : avertir à l'écriture d'un document déjà signé numériquement (toutes tranches concernées) : tranche transversale ou `security-signatures` ?
* **Q5** : import XFDF/FDF voulu ? (`quick-xml` 0.41 MIT, API à réétudier : `Event::GeneralRef`) ; sinon JSON suffit.
* **Q6** : raccourcis : Ctrl+Maj+F pour le panneau, aucun pour l'outil (F, K, S, F5 pris par d'autres plans) ; accepter ?
* **Q7** : ajouter `serde_json` au crate `sunipdf-core` (déjà dans le workspace) ?
* **Q8** : étapes 11 (création de champs) et 12 (XFDF export) : à planifier dans cette livraison ou plus tard ?
