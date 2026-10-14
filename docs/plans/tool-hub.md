# Tranche « tool-hub » : hub « Tous les outils », barre d'outil contextuelle, polish du chrome

Statut : plan d'implémentation (aucun code écrit, aucune commande `cargo` lancée pour le produire).
Auteur : agent ui-ux. Date : 2026-10-14. Base : SuniPDF 0.3.0, `main`, 63 commandes au registre, 39 tests verts.
Propriétaire de l'implémentation : ui-ux (fichiers de `apps/sunipdf-desktop/**`, catalogues i18n). La seule modification
hors de ce périmètre est `crates/sunipdf-core/src/command.rs` (registre de commandes), à valider par l'architect (§3).

Conventions : les chemins de dépendances sont relatifs à
`C:/Users/Emanuel/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` ; « egui » = `egui-0.36.2/src`,
« epaint » = `epaint-0.36.2/src`, « ecolor » = `ecolor-0.36.2/src`. Toute API citée a été relue dans ces sources
(index des citations en annexe A). `docs/PDFCRAFT_AUDIT.md` ne décrit aucune capture d'écran : la cible visuelle vient de la
liste « All tools » du cahier des charges et du code existant, pas d'images de PDFCraft.

---

## 0. Décisions de conception

1. Un **outil** (`Workspace`, 14 variantes = la liste PDFCraft) regroupe des **commandes du registre existant**. Aucune
   logique PDF nouvelle, aucune API moteur, aucun `Request` de worker.
2. Table de données unique `workspace::TABLE` (même esprit que `REGISTRY`) : nom, description, icône, couleur, **entrée**,
   liste de commandes. Un outil dont `entry == None` n'apparaît **nulle part** (ni panneau, ni accueil, ni menu) : c'est la
   règle « pas de bouton factice ». Aujourd'hui 9 outils visibles sur 14.
3. Deux types d'entrée : `Mode` (barre contextuelle ; outils Modifier, Organiser, Commenter, Protéger, Tampon) et `Direct`
   (lance la boîte de dialogue existante ; Exporter, Créer, Combiner, Compresser). Un outil à une seule commande n'a pas de
   mode : une barre à un bouton serait un clic de trop.
4. Une commande n'a **qu'un seul outil propriétaire** (test). Le propriétaire fixe sa couleur partout (panneau, palette
   Ctrl+K, barre) et permet de retrouver l'outil depuis un raccourci (H ouvre « Commenter »).
5. L'outil actif vit dans `AnnotateState.workspace` : il suit donc l'onglet (échange `take_tab`/`put_tab`) sans toucher
   `tabs.rs`, et se remet à zéro à la fermeture du document.
6. `ToolTab` (7 onglets texte de la barre d'outils + liste plate de ~30 commandes) **disparaît**. Le panneau de gauche devient
   le hub (liste des outils). La barre d'outils principale gagne un accès rapide (Ouvrir, Enregistrer, Annuler, Rétablir)
   et un bouton à bascule « Tous les outils ».
7. La palette flottante verticale (`tool_palette`) est **supprimée** : ses outils passent dans la barre contextuelle. La barre
   d'objet flottante reste, réduite aux **propriétés** (trait, remplissage, Appliquer) ; ses six boutons d'action passent dans
   la barre contextuelle (désactivés sans sélection).
8. Quitter un outil : bouton « Quitter l'outil » à droite de la barre, bouton retour « ‹ » à gauche, **Échap**, nouveau clic sur
   la ligne active du panneau, commande `hub.leave` (palette, menu Outils).
9. Clavier : F9 (panneau), Tab / flèches / Entrée dans le panneau (focus natif egui), Ctrl+K (une entrée `hub.*` par outil à
   mode), Échap. Aucun raccourci chiffré (AZERTY : Ctrl+1 n'existe pas sans Maj).
10. Polish : jetons d'espacement/rayon/typo, correction de **deux défauts de contraste mesurés** (bouton plein en thème sombre
    2,54:1 ; glyphes de catégorie 1,9:1 à 2,6:1), survol animé, anneau de focus, cartes d'accueil entièrement cliquables.

---

## 1. Objectif et comportement visible

### 1.1 Correspondance avec les outils PDFCraft

| Outil PDFCraft | `Workspace` | Entrée | Commandes du registre (propriétaire unique) | Visible aujourd'hui |
|---|---|---|---|---|
| Export a PDF | `Export` | Direct | `ExportImages` | oui (PNG 150 ppp seulement) |
| Fill & Sign | `FillSign` | aucune | rien | **non** (formulaires/signature image : autres tranches) |
| Edit a PDF | `Edit` | Mode `hub.edit` | `ToolSelectObject, ToolAddText, ToolAddImage, ToolRectangle, ToolEllipse, ToolLine` \| `EditObjectText` \| `DuplicateObject, RotateObjectLeft, RotateObjectRight, BringToFront, SendToBack, DeleteObject` | oui |
| Create a PDF | `Create` | Direct | `CreateFromImages` | oui (images → PDF ; pas de page vierge) |
| Combine files | `Combine` | Direct | `CombineFiles` | oui |
| Organize pages | `Organize` | Mode `hub.organize` | `OrganizePages` (grille) \| `RotateCounterClockwise, RotateClockwise, DeletePage, InsertPages, MovePageLeft, MovePageRight` \| `SplitDocument, ExtractPages` | oui |
| Add comments | `Comment` | Mode `hub.comment` | `ToolSelect, ToolHighlight, ToolUnderline, ToolStrikeOut, ToolNote` \| `ToggleComments, CopyText` | oui |
| Scan & OCR | `ScanOcr` | aucune | rien | **non** (tranche OCR) |
| Protect a PDF | `Protect` | Mode `hub.protect` | `Protect, Unprotect, StripMetadata, Properties` | oui |
| Redact a PDF | `Redact` | aucune | rien | **non** (tranche caviardage) |
| Compress a PDF | `Compress` | Direct | `Optimize` | oui (sans perte seulement) |
| Prepare a form | `PrepareForm` | aucune | rien | **non** |
| Add a stamp | `Stamp` | Mode `hub.stamp` | `Watermark, PageNumbers` | oui |
| Use a certificate | `Certificate` | aucune | rien | **non** (signatures numériques : phase P5) |

Commandes volontairement **hors outils** (système, vue, navigation ; liste `SYSTEM_COMMANDS` du test, §8.2) :
`OpenDocument, CloseDocument, Quit, Save, SaveAs, Undo, Redo, Find, FirstPage, PreviousPage, NextPage, LastPage, ZoomIn, ZoomOut,
ActualSize, FitWidth, FitPage, ToggleThumbnails, ToggleBookmarks, ToggleTools, CommandPalette, NextTab, PrevTab, About` et les 6
commandes `Hub*`.

### 1.2 Ce que l'utilisateur voit

**Accueil (aucun document).** Les sept cartes « Lire / Rechercher / Pages / Annoter / Sécuriser / Convertir / Éditer » sont
remplacées par une carte pleine « Ouvrir un fichier » (accent), puis une carte par outil visible (9), dans l'ordre PDFCraft :
Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger, Compresser, Tampon. Chaque carte (244 × 120 px) porte la
pastille colorée de l'outil, son nom, sa description sur deux lignes ; **toute la carte est cliquable** (aujourd'hui seul le lien
« Ouvrir un PDF… » l'est). Clic = `execute(commande d'entrée)`. Pour un outil qui exige un document, le sélecteur de fichiers
s'ouvre d'abord puis l'outil démarre (mécanisme `after_open` existant, `app.rs:793-800`). « Combiner » et « Créer »
n'exigent pas de document.

**Panneau de gauche « Tous les outils » (F9).** Neuf lignes de 40 px au lieu de ~30 commandes : pastille 28 px colorée, nom,
chevron `›` pour les outils à mode. Survol : fondu 100 ms. Outil actif : fond `accent_soft`, trait d'accent de 3 px à gauche dans la
couleur de l'outil, nom en semi-gras. Info-bulle = description. Clic sur la ligne active = quitter l'outil. Le pied de panneau
garde le rappel `tools.hint` (texte mis à jour).

**Barre d'outil contextuelle** (nouvelle `Panel::top("workspace")`, 46 px, entre la barre d'outils et la barre de recherche),
visible seulement si un document est affiché ET qu'un outil à mode est actif :

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  (filet de 2 px, couleur de l'outil)
[‹]  [■ Commenter]  │ [I Sélectionner du texte] [▬ Surligner] [_ Souligner] [S Barrer] [✎ Note] │ [💬 Commentaires] [⎘ Copier]        [✕ Quitter l'outil  Échap]
```

- Les commandes sont les `commands` de l'outil, groupées : outils de dessin/sélection (ceux de `annotate::Tool`), séparateur,
  actions.
- Chaque bouton : icône 16 px colorée + libellé **si la largeur le permet** (calcul dynamique, §5.5), sinon icône seule ;
  info-bulle `libellé  (raccourci)` toujours présente.
- Bouton sélectionné (outil actif, grille ouverte, panneau commentaires ouvert) : fond `accent_soft` + liseré de 2 px dans la
  couleur de l'outil.
- Bouton indisponible (aucun objet sélectionné, pas de texte sélectionné, aucune page sélectionnée dans la grille) : grisé à 40 %,
  info-bulle conservée.
- Entrer dans « Organiser » ouvre la grille (comme F8) ; la barre porte alors les actions de page, et la barre interne de la
  grille ne garde que « N / M page(s) sélectionnée(s) » et « Tout sélectionner ».
- Entrer dans « Modifier » active l'outil « Sélectionner un objet » ; dans « Commenter », la sélection de texte.

**Quitter.** Retour à la lecture : outil de dessin = sélection de texte, objet désélectionné, grille fermée si elle venait de
l'outil Organiser. Les panneaux ouverts à la main (commentaires, signets, miniatures) ne sont pas fermés.

**Raccourcis existants conservés.** H/U/X/N/E/T/I/R/O/L, V, F2, Suppr, Ctrl+] / Ctrl+[ ne changent pas ; ils **suivent** la barre :
H sans outil actif ouvre « Commenter » avec Surligner sélectionné ; E/T/I/R/O/L ouvrent « Modifier » ; V quitte l'outil courant
s'il ne propose pas la sélection de texte (V = retour au curseur de lecture) ; F8 ouvre la grille *et* l'outil Organiser.

**Menus et palette.** Nouveau menu « Outils » dans le bouton Menu (un élément par outil visible, coché si actif, puis « Quitter
l'outil »). La palette Ctrl+K liste les 5 entrées `hub.*` (couleur et icône de l'outil) + `hub.leave`.

### 1.3 Ce qui disparaît

- `ToolTab` et les 7 onglets texte de `toolbar()` (clés `toolbar.read|annotate|edit|pages|security|convert`).
- La palette flottante `annotate::tool_palette` (appel `app.rs:1230`).
- Les 6 boutons d'action de `object_bar` (`editor.rs:643-663`) ; les boutons Pivoter/Supprimer/Insérer/Déplacer/Fermer de
  `organize_toolbar` (`organize.rs:55-95`).
- Les 7 cartes `home.card.*` et le lien `home.card.use`.
- `shell::tool_row` (remplacé par `hub_row`).

---

## 2. Faisabilité

**Verdict : oui** pour le hub, la barre contextuelle, la navigation clavier et le polish (100 % UI, 0 dépendance).
**Partiel par construction** sur la liste PDFCraft : 9 outils sur 14 sont visibles, car 5 n'ont aucune commande réelle.

Volontairement hors tranche, avec la raison :

| Exclu | Pourquoi |
|---|---|
| Fill & Sign, Scan & OCR, Redact, Prepare a form, Use a certificate | Aucune fonction réelle dans le moteur ; les afficher serait un bouton factice (CLAUDE.md). Leur entrée dans la table est `None` ; chaque tranche concernée la renseigne (§5.9). |
| Mode (barre) pour Exporter, Créer, Combiner, Compresser | Une seule commande chacun, déjà une boîte de dialogue : le mode ajouterait un clic. Passage en `Mode` quand l'outil gagne des options (niveaux de compression, export texte...). |
| Mémorisation du dernier outil / outils favoris | Besoin non démontré (YAGNI). |
| Recherche d'outil dans le panneau | La palette Ctrl+K couvre le besoin. |
| Panneau réduit en rail d'icônes | Gain de place réel mais hors périmètre ; le panneau se ferme déjà avec F9. |
| Test de rendu au pixel de l'interface | Pas de rendu logiciel ni d'`egui_kittest` dans les dépendances. Remplacé par : tests de données purs, test headless de mise en page + arbre AccessKit (§8.3), captures inspectées (§8.5). Limite à consigner dans `KNOWN_ISSUES.md`. |
| Vérification visuelle des glyphes de repli (sans Segoe Fluent Icons) | Les symboles de repli des nouvelles icônes ne sont pas vérifiés visuellement (déjà dit dans `KNOWN_ISSUES.md` A1). |

Honnêteté sur les glyphes : la **présence** des points de code `EDE1, E70F, E710, E76C, E72B` dans
`C:\Windows\Fonts\SegoeIcons.ttf` a été vérifiée sur ce poste (PowerShell, `System.Windows.Media.GlyphTypeface.CharacterToGlyphMap`) ;
leur **apparence** doit être confirmée par capture (étape 5).

---

## 3. API moteur (`sunipdf-core`)

### 3.1 Aucune API PDF

Le hub ne lit ni n'écrit de PDF : toutes les actions ciblées existent déjà (`execute` → `Request`/`EditOp`/`BatchOp`). Aucune
signature de `Engine`/`Document`, aucun appel `pdfium-render` ou `lopdf`, aucun `regenerate_content`, aucun instantané à ajouter.
Le basculement d'outil n'est **pas** une modification de document : il n'entre pas dans le journal d'annulation et ne marque
pas le document « modifié ».

### 3.2 Seul changement dans le crate : le registre (`crates/sunipdf-core/src/command.rs`)

Pourquoi dans le crate cœur : `Command` y est défini, et `execute`, la palette, la CLI (`commands`) et l'automatisation
(`SUNIPDF_STARTUP_COMMANDS`, utilisée par `scripts/dev-screenshot.ps1 -Commands`) lisent le même tableau (en-tête du fichier).
Données pures, aucune dépendance UI.

1. `Menu` (`command.rs:72`) : ajouter la variante `Tools`.
2. `Command` (`command.rs:5-68`) : ajouter, avant `About`, les variantes `HubEdit, HubOrganize, HubComment, HubProtect, HubStamp,
   HubLeave`.
3. `REGISTRY` (`command.rs:109-607`) : ajouter, avant l'entrée `About` (`command.rs:606`) :

```rust
cmd!(HubEdit,     "hub.edit",     "hub.edit.name",     None, true,  Tools),
cmd!(HubOrganize, "hub.organize", "hub.organize.name", None, true,  Tools),
cmd!(HubComment,  "hub.comment",  "hub.comment.name",  None, true,  Tools),
cmd!(HubProtect,  "hub.protect",  "hub.protect.name",  None, true,  Tools),
cmd!(HubStamp,    "hub.stamp",    "hub.stamp.name",    None, true,  Tools),
cmd!(HubLeave,    "hub.leave",    "hub.leave", Some("shortcut.hub_leave"), false, Tools),
```

   (`needs_document = true` pour les cinq outils à mode : `execute` ouvre alors d'abord le sélecteur de fichiers, mécanisme
   `after_open`. `hub.leave` n'exige pas de document : sans effet quand aucun outil n'est actif.)
4. Tests de `command.rs` (`command.rs:632-649`) : ajouter `assert_eq!(Command::in_menu(Menu::Tools).count(), 6);` et un test
   `hub_ids_use_hub_prefix` (tout id de `Menu::Tools` commence par `hub.`).

Effets de bord à connaître :
- `sunipdf-cli commands` (`apps/sunipdf-cli/src/main.rs:265-275`) imprime `"menu":"tools"` pour ces 6 entrées, comme il imprime
  déjà des commandes d'interface pures (`view.tools`, `app.palette`). Aucun changement de CLI (§6).
- Deux `match` exhaustifs du desktop doivent recevoir les nouvelles variantes dans le **même commit** : `execute`
  (`app.rs:793-1009`, pas de bras par défaut) et `shell::command_icon` (`shell.rs:246`).

---

## 4. Worker

**Aucune variante** `Request`, `Response`, `EditOp`, `BytesOp` ou `BatchOp` à ajouter ; `worker.rs` n'est pas modifié.

Points de contact à vérifier (non modifiés) :
- Les actions des barres appellent `SuniApp::execute`, qui envoie déjà les `Request::Edit(EditOp::…)` existants (rotation,
  suppression, insertion, déplacement, opérations d'objets) : même chemin, même instantané d'annulation.
- Le thread UI n'appelle toujours aucune opération PDF bloquante : la barre ne fait que dessiner et émettre des commandes.
- Les boîtes de dialogue de fichiers (`rfd`) restent synchrones (limite connue A5 de `KNOWN_ISSUES.md`).

---

## 5. Interface

### 5.1 Fichiers à créer / modifier

| Fichier | Action |
|---|---|
| `apps/sunipdf-desktop/src/workspace.rs` | **nouveau** : table des outils, bascule d'état pure, barre contextuelle, `bar_button`, `command_enabled`, tests |
| `apps/sunipdf-desktop/src/main.rs` | `mod workspace;` |
| `apps/sunipdf-desktop/src/annotate.rs` | champ `workspace` dans `AnnotateState` ; `Tool::from_command` ; suppression de `tool_palette` |
| `apps/sunipdf-desktop/src/app.rs` | suppression de `tool_tab` ; bras `Hub*` ; `follow` et Échap ; `Panel::top("workspace")` ; liste d'arrêt de `handle_input` ; champ `focus_last_frame` ; `menu_item` en `pub(crate)` |
| `apps/sunipdf-desktop/src/shell.rs` | suppression de `ToolTab`/`tool_row` ; `hub_row`, `badge`, `ghost_icon`, `leave_button` ; `tools_panel`, `toolbar`, `home`, `command_color`, `command_icon` réécrits ; onglet actif (trait d'accent) |
| `apps/sunipdf-desktop/src/editor.rs` | `object_bar` : retrait des 6 boutons d'action |
| `apps/sunipdf-desktop/src/organize.rs` | `organize_toolbar` réduite (compteur + « Tout sélectionner ») |
| `apps/sunipdf-desktop/src/theme.rs` | jetons, 3 champs de `Palette`, 4 helpers, réglages `apply()` |
| `apps/sunipdf-desktop/src/dialog.rs` | `primary` : fond `p.accent_fill` (contraste) |
| `apps/sunipdf-desktop/src/icons.rs` | 5 glyphes |
| `crates/sunipdf-core/src/command.rs` | §3.2 |
| `assets/i18n/fr-FR.json`, `en-US.json` | §7 |
| `scripts/dev-screenshot.ps1` | uniquement l'exemple de l'en-tête (ids `hub.*`) |

`tabs.rs`, `docinfo.rs`, `tools.rs`, `worker.rs` : **inchangés**.

### 5.2 Modèle de données (`workspace.rs`)

```rust
//! Hub « Tous les outils » : regroupe les commandes du registre par tâche (liste de tâches façon PDFCraft).
//! Données pures et bascule d'état testables sans fenêtre ; le dessin des lignes/cartes est dans `shell.rs`.

use crate::annotate::{AnnotateState, Tool};
use crate::editor::EditState;
use crate::icons::glyph;
use crate::organize::OrganizeState;
use egui::Color32;
use sunipdf_core::Command;

/// Ordre = ordre de la liste « All tools » de PDFCraft ; `TABLE[w as usize].ws == w` (testé).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Workspace {
    Export, FillSign, Edit, Create, Combine, Organize, Comment,
    ScanOcr, Protect, Redact, Compress, PrepareForm, Stamp, Certificate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// Un clic lance la commande (boîte de dialogue / sélecteur de fichier). Pas de mode, pas de barre.
    Direct(Command),
    /// Un clic entre dans le mode. La commande est l'entrée `hub.*` du registre ; `commands` remplit la barre.
    Mode(Command),
}

pub struct WsInfo {
    pub ws: Workspace,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    /// (glyphe Segoe Fluent, repli texte), comme `shell::command_icon`.
    pub icon: (&'static str, &'static str),
    pub color: Color32,
    /// `None` = non implémenté : l'outil n'est affiché nulle part.
    pub entry: Option<Entry>,
    /// Contenu de la barre (Mode) ; `[commande]` pour un Direct ; `&[]` si `entry` est `None`.
    pub commands: &'static [Command],
}
```

La table (couleurs : §5.11.3) :

```rust
use Command as C;
pub const TABLE: [WsInfo; 14] = [
    WsInfo { ws: Workspace::Export, name_key: "hub.export.name", desc_key: "hub.export.desc",
        icon: (glyph::EXPORT, "↗"), color: Color32::from_rgb(0x0E, 0xA5, 0xE9),
        entry: Some(Entry::Direct(C::ExportImages)), commands: &[C::ExportImages] },
    WsInfo { ws: Workspace::FillSign, name_key: "hub.fill_sign.name", desc_key: "hub.fill_sign.desc",
        icon: (glyph::EDIT_DOC, "✎"), color: Color32::from_rgb(0x63, 0x66, 0xF1), entry: None, commands: &[] },
    WsInfo { ws: Workspace::Edit, name_key: "hub.edit.name", desc_key: "hub.edit.desc",
        icon: (glyph::EDIT_DOC, "✎"), color: Color32::from_rgb(0xE1, 0x1D, 0x48),
        entry: Some(Entry::Mode(C::HubEdit)),
        commands: &[C::ToolSelectObject, C::ToolAddText, C::ToolAddImage, C::ToolRectangle, C::ToolEllipse,
                    C::ToolLine, C::EditObjectText, C::DuplicateObject, C::RotateObjectLeft,
                    C::RotateObjectRight, C::BringToFront, C::SendToBack, C::DeleteObject] },
    WsInfo { ws: Workspace::Create, name_key: "hub.create.name", desc_key: "hub.create.desc",
        icon: (glyph::ADD, "+"), color: Color32::from_rgb(0x16, 0xA3, 0x4A),
        entry: Some(Entry::Direct(C::CreateFromImages)), commands: &[C::CreateFromImages] },
    WsInfo { ws: Workspace::Combine, name_key: "hub.combine.name", desc_key: "hub.combine.desc",
        icon: (glyph::COMBINE, "⧉"), color: Color32::from_rgb(0x25, 0x63, 0xEB),
        entry: Some(Entry::Direct(C::CombineFiles)), commands: &[C::CombineFiles] },
    WsInfo { ws: Workspace::Organize, name_key: "hub.organize.name", desc_key: "hub.organize.desc",
        icon: (glyph::PAGES, "▦"), color: Color32::from_rgb(0xF5, 0x9E, 0x0B),
        entry: Some(Entry::Mode(C::HubOrganize)),
        commands: &[C::OrganizePages, C::RotateCounterClockwise, C::RotateClockwise, C::DeletePage,
                    C::InsertPages, C::MovePageLeft, C::MovePageRight, C::SplitDocument, C::ExtractPages] },
    WsInfo { ws: Workspace::Comment, name_key: "hub.comment.name", desc_key: "hub.comment.desc",
        icon: (glyph::COMMENT, "💬"), color: Color32::from_rgb(0x8B, 0x5C, 0xF6),
        entry: Some(Entry::Mode(C::HubComment)),
        commands: &[C::ToolSelect, C::ToolHighlight, C::ToolUnderline, C::ToolStrikeOut, C::ToolNote,
                    C::ToggleComments, C::CopyText] },
    WsInfo { ws: Workspace::ScanOcr, name_key: "hub.scan_ocr.name", desc_key: "hub.scan_ocr.desc",
        icon: (glyph::PICTURE, "🖼"), color: Color32::from_rgb(0x06, 0xB6, 0xD4), entry: None, commands: &[] },
    WsInfo { ws: Workspace::Protect, name_key: "hub.protect.name", desc_key: "hub.protect.desc",
        icon: (glyph::LOCK, "🔒"), color: Color32::from_rgb(0x0D, 0x94, 0x88),
        entry: Some(Entry::Mode(C::HubProtect)),
        commands: &[C::Protect, C::Unprotect, C::StripMetadata, C::Properties] },
    WsInfo { ws: Workspace::Redact, name_key: "hub.redact.name", desc_key: "hub.redact.desc",
        icon: (glyph::ERASE, "⌫"), color: Color32::from_rgb(0x47, 0x55, 0x69), entry: None, commands: &[] },
    WsInfo { ws: Workspace::Compress, name_key: "hub.compress.name", desc_key: "hub.compress.desc",
        icon: (glyph::OPTIMIZE, "⤓"), color: Color32::from_rgb(0xEA, 0x58, 0x0C),
        entry: Some(Entry::Direct(C::Optimize)), commands: &[C::Optimize] },
    WsInfo { ws: Workspace::PrepareForm, name_key: "hub.prepare_form.name", desc_key: "hub.prepare_form.desc",
        icon: (glyph::RECT, "▭"), color: Color32::from_rgb(0x65, 0xA3, 0x0D), entry: None, commands: &[] },
    WsInfo { ws: Workspace::Stamp, name_key: "hub.stamp.name", desc_key: "hub.stamp.desc",
        icon: (glyph::STAMP, "◈"), color: Color32::from_rgb(0xC0, 0x26, 0xD3),
        entry: Some(Entry::Mode(C::HubStamp)), commands: &[C::Watermark, C::PageNumbers] },
    WsInfo { ws: Workspace::Certificate, name_key: "hub.certificate.name", desc_key: "hub.certificate.desc",
        icon: (glyph::SHIELD, "◇"), color: Color32::from_rgb(0xDB, 0x27, 0x77), entry: None, commands: &[] },
];
```

(Les icônes des outils masqués sont provisoires ; la tranche qui active l'outil choisit la sienne.)

Fonctions de lecture :

```rust
impl Workspace {
    pub const ALL: [Workspace; 14] = [ /* les 14 variantes dans l'ordre de l'enum */ ];
    pub fn info(self) -> &'static WsInfo { &TABLE[self as usize] }
    /// Seuls les outils réels : jamais de bouton factice.
    pub fn available() -> impl Iterator<Item = Workspace> {
        Self::ALL.into_iter().filter(|w| w.info().entry.is_some())
    }
    /// Commande exécutée par un clic sur la ligne / la carte.
    pub fn activate(self) -> Option<Command> {
        match self.info().entry? { Entry::Direct(c) | Entry::Mode(c) => Some(c) }
    }
    pub fn is_mode(self) -> bool { matches!(self.info().entry, Some(Entry::Mode(_))) }
    /// `Command::HubEdit` -> `Workspace::Edit`.
    pub fn from_command(c: Command) -> Option<Workspace> {
        Self::available().find(|w| w.info().entry == Some(Entry::Mode(c)))
    }
    /// Propriétaire (unique) d'une commande du registre.
    pub fn owner_of(c: Command) -> Option<Workspace> {
        Self::available().find(|w| w.info().commands.contains(&c))
    }
}
```

`Color32::from_rgb` est `const` (même usage dans `theme.rs:6-15`). `Command` dérive `PartialEq, Eq, Copy` (`command.rs:4`),
donc `Entry` aussi.

### 5.3 État et bascule (fonctions pures, testables sans `SuniApp`)

`annotate.rs` :

```rust
pub struct AnnotateState {            // annotate.rs:88
    pub tool: Tool,
    /* ...champs existants... */
    /// Outil actif du hub. Dans `AnnotateState` pour suivre l'onglet sans modifier `tabs.rs`
    /// (`take_tab`/`put_tab` échangent `annotate`, tabs.rs:69 et :99) et se vider à la fermeture (`close_document`).
    pub workspace: Option<crate::workspace::Workspace>,
}
impl Tool {
    pub fn from_command(c: Command) -> Option<Tool> { Tool::ALL.into_iter().find(|t| t.command() == c) }
}
```

`workspace.rs` (fonctions libres ; `AnnotateState`, `OrganizeState`, `EditState` ont toutes `Default`) :

```rust
/// Change d'outil. Ferme la grille si on quitte Organiser ; abandonne l'outil de dessin s'il n'appartient pas au nouvel outil.
pub fn switch(a: &mut AnnotateState, o: &mut OrganizeState, e: &mut EditState, to: Option<Workspace>) {
    let from = std::mem::replace(&mut a.workspace, to);
    if from == to { return; }
    if from == Some(Workspace::Organize) { o.active = false; }
    let keep = to.is_some_and(|w| w.info().commands.contains(&a.tool.command()));
    if !keep { a.tool = Tool::Select; a.drag = None; e.selected = None; }
}

/// Garde la barre cohérente quand une commande-outil ou la grille est lancée hors du hub (raccourci, palette, menu, rail).
pub fn follow(a: &mut AnnotateState, o: &mut OrganizeState, e: &mut EditState, command: Command) {
    if let Some(tool) = Tool::from_command(command) {
        let to = if tool == Tool::Select {
            // V = retour au curseur de lecture : on reste dans l'outil seulement s'il propose la sélection de texte.
            a.workspace.filter(|w| w.info().commands.contains(&command))
        } else {
            Workspace::owner_of(command).or(a.workspace)
        };
        switch(a, o, e, to);
    } else if command == Command::OrganizePages && o.active {
        switch(a, o, e, Some(Workspace::Organize));
    }
}
```

`impl SuniApp` (dans `workspace.rs`) :

```rust
pub fn enter_workspace(&mut self, ctx: &egui::Context, ws: Workspace) {
    switch(&mut self.annotate, &mut self.organize, &mut self.edit, Some(ws));
    // Premier outil de dessin de la liste = outil par défaut (Sélectionner un objet / Sélectionner du texte).
    if let Some(first) = ws.info().commands.iter().copied().find(|c| Tool::from_command(*c).is_some()) {
        self.execute(ctx, first);
    }
    if ws == Workspace::Organize && !self.organize.active { self.execute(ctx, Command::OrganizePages); }
}
pub fn leave_workspace(&mut self) { switch(&mut self.annotate, &mut self.organize, &mut self.edit, None); }
/// Échap quitte l'outil seulement si rien de plus proche ne l'utilise : recherche ouverte, objet sélectionné,
/// ou un champ qui avait le focus à l'image précédente (egui efface le focus sur Échap avant que l'application lise la touche).
pub fn esc_leaves_workspace(&self) -> bool {
    self.annotate.workspace.is_some() && !self.find.open && self.edit.selected.is_none() && !self.focus_last_frame
}
```

Branchements dans `app.rs` :

1. `execute`, bras `Command::ToolSelect | … | ToolLine` (`app.rs:923-941`) : à la fin du bras, ajouter
   `crate::workspace::follow(&mut self.annotate, &mut self.organize, &mut self.edit, command);`. Bras `Command::OrganizePages`
   (`app.rs:986-997`) : même appel après la bascule de `organize.active`.
2. Nouveaux bras (exhaustivité) :

```rust
Command::HubEdit | Command::HubOrganize | Command::HubComment | Command::HubProtect | Command::HubStamp => {
    if let Some(ws) = crate::workspace::Workspace::from_command(command) { self.enter_workspace(ctx, ws); }
}
Command::HubLeave => self.leave_workspace(),
```
3. `handle_input` :
   - ajouter `|| self.docinfo.properties_open || self.docinfo.protect_open` à la condition d'arrêt `app.rs:591-597` : ces deux
     dialogues lisent déjà Échap eux-mêmes (`docinfo.rs:209`, `:290`) et ne bloquent pas aujourd'hui les raccourcis ; sans cela
     Échap fermerait la boîte **et** quitterait l'outil ;
   - avant `ctx.input_mut` (`app.rs:~700`) : `let leave_on_esc = self.esc_leaves_workspace();` puis, dans la liste des touches :
     `if leave_on_esc { key(Modifiers::NONE, Key::Escape, Command::HubLeave); }`.
   Le bloc « objet sélectionné » (`app.rs:~680-695`) consomme déjà Échap (`consume_key`) pour désélectionner : la désélection a
   priorité, un second Échap quitte l'outil.
4. **Piège du focus** : egui efface le focus sur Échap au début de la passe (`egui/memory/mod.rs:595-598`, `begin_pass`), avant que
   `handle_input` ne lise la touche ; la branche « un champ a le focus » (`app.rs:~603`) ne protège donc plus. Sans garde, Échap dans le
   champ « numéro de page » du rail (ou sur une ligne du hub focalisée) quitterait l'outil. Parade : champ `focus_last_frame: bool`
   dans `SuniApp`, mis à jour en fin de `ui()` par `self.focus_last_frame = ctx.memory(|m| m.focused().is_some());`, lu par
   `esc_leaves_workspace`. Un Échap = un effet (d'abord le focus, puis l'outil).
5. `SuniApp::new` : supprimer `tool_tab` (`app.rs:115`, `:139`), ajouter `focus_last_frame: false`.

Onglets et annulation :
- **Onglets** : `take_tab` (`tabs.rs:69`) emporte `annotate` (donc `workspace`) et `put_tab` (`tabs.rs:99`) le restitue : chaque onglet
  garde son outil et son outil de dessin cohérents. Un document ouvert dans un nouvel onglet démarre sans outil
  (`AnnotateState::default()`), puis `after_open` peut en entrer un.
- **Annuler/Rétablir** : le changement d'outil n'est pas une opération du journal. Les actions de la barre passent par `execute`, donc par
  `EditOp` sous instantané, inchangé.
- **Fermeture du document** : `close_document` (`app.rs:~187`) fait déjà `self.annotate = Default::default()`.

### 5.4 Commandes ajoutées

| Id | Clé de libellé | Raccourci | Menu | Document requis | Effet |
|---|---|---|---|---|---|
| `hub.edit` | `hub.edit.name` | aucun | Tools | oui | entre dans Modifier |
| `hub.organize` | `hub.organize.name` | aucun (F8 ouvre la grille *et* l'outil) | Tools | oui | entre dans Organiser + grille |
| `hub.comment` | `hub.comment.name` | aucun | Tools | oui | entre dans Commenter |
| `hub.protect` | `hub.protect.name` | aucun | Tools | oui | entre dans Protéger |
| `hub.stamp` | `hub.stamp.name` | aucun | Tools | oui | entre dans Tampon |
| `hub.leave` | `hub.leave` | `shortcut.hub_leave` = Échap | Tools | non | quitte l'outil actif |

Les outils Direct n'ont **pas** de commande `hub.*` : leur ligne ou carte exécute directement la commande existante (`file.combine`,
`convert.optimize`, `convert.export_images`, `convert.images_to_pdf`), déjà dans la palette.

### 5.5 Barre contextuelle (`workspace.rs`)

Intégration dans `eframe::App::ui` (`app.rs:1761`), **après** `Panel::top("toolbar")` (`app.rs:1776`) et avant `"find"` (`:1785`) :

```rust
if self.document.is_some() && self.annotate.workspace.is_some_and(|w| w.is_mode()) {
    egui::Panel::top("workspace")
        .exact_size(46.0) // pas de saut de hauteur quand les libellés apparaissent/disparaissent (egui/containers/panel.rs:405)
        .frame(egui::Frame::new().fill(p.surface).stroke(egui::Stroke::new(1.0, p.border))
            .inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 6 }))
        .show(ui, |ui| self.workspace_bar(&ctx, ui));
}
```

Squelette de `workspace_bar` (même schéma `run: Option<Command>` + exécution après la closure que `tools_panel` actuel,
`shell.rs:578-632`, pour ne pas emprunter `self` deux fois). Helpers à écrire dans `shell.rs` : `badge(ui, has_icons, icon, color,
size, dark)` (pastille colorée, partagée par la barre, les lignes et les cartes), `ghost_icon(ui, has_icons, icon, tooltip)`
(bouton icône sans cadre 28 px, libellé AccessKit = `tooltip`), `leave_button(ui, has_icons, label, shortcut)` (✕ + libellé +
raccourci en texte atténué) :

```rust
pub fn workspace_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let Some(ws) = self.annotate.workspace else { return };
    let info = ws.info();
    let (lang, dark, p) = (self.lang(), self.settings.dark_theme, self.pal());
    let labels = self.bar_fits_labels(ui, info.commands);
    let (mut run, mut back) = (None::<Command>, false);
    // Filet de 2 px dans la couleur de l'outil, premier élément (donc collé au bord haut du cadre sans marge haute).
    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, info.color);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        back = crate::shell::ghost_icon(ui, self.has_icons, (glyph::ARROW_LEFT, "‹"), tr(lang, "hub.back")).clicked();
        crate::shell::badge(ui, self.has_icons, info.icon, info.color, 28.0, dark);
        ui.label(icons::semibold(tr(lang, info.name_key), 15.0).color(p.text));
        ui.separator();
        let mut prev_is_tool = None;
        for &c in info.commands {
            let is_tool = Tool::from_command(c).is_some();
            if prev_is_tool.is_some_and(|t| t != is_tool) { ui.separator(); }
            prev_is_tool = Some(is_tool);
            let i = c.info();
            let tip = match i.shortcut_key {
                Some(k) => format!("{}  ({})", tr(lang, i.label_key), tr(lang, k)),
                None => tr(lang, i.label_key).to_string(),
            };
            let selected = self.command_selected(c);
            // L'outil de dessin sélectionné montre toujours son libellé, même quand la barre est en icônes seules.
            let r = bar_button(ui, self.has_icons, crate::shell::command_icon(c), info.color,
                               tr(lang, i.label_key), labels || (selected && is_tool), &tip, selected,
                               self.command_enabled(c));
            if r.clicked() { run = Some(c); r.surrender_focus(); }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let sc = tr(lang, "shortcut.hub_leave");
            if crate::shell::leave_button(ui, self.has_icons, tr(lang, "hub.leave"), sc).clicked() {
                run = Some(Command::HubLeave);
            }
        });
    });
    if back { self.show_tools = true; self.settings.show_tools = true; self.persist_settings(); run = Some(Command::HubLeave); }
    if let Some(c) = run { self.execute(ctx, c); }
}
```

`bar_button` (fonction libre, dessin personnalisé pour obtenir fondu, état sélectionné, anneau de focus et libellé AccessKit
propre : un `egui::Button` contenant un glyphe de la zone privée exposerait le caractère brut aux lecteurs d'écran) :

```rust
pub fn bar_button(ui: &mut egui::Ui, has_icons: bool, icon: (&str, &str), tint: Color32, label: &str,
                  show_label: bool, tooltip: &str, selected: bool, enabled: bool) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let p = theme::palette(dark);
    let alpha = if enabled { 1.0 } else { 0.4 };
    // Mesure du libellé : Painter::layout_no_wrap (egui/painter.rs:503) + Galley::size (epaint/text/text_layout_types.rs:1017).
    let galley = show_label.then(|| ui.painter().layout_no_wrap(
        label.to_string(), egui::FontId::proportional(14.0), p.text.gamma_multiply(alpha)));
    let w = 34.0 + galley.as_ref().map_or(0.0, |g| g.size().x + 8.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 34.0),
        if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    // AccessKit : libellé lisible + état (Response::widget_info, egui/response.rs:869 -> set_label / set_toggled :962-980).
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, enabled, selected, label));
    let resp = resp.on_hover_text(tooltip.to_string()).on_disabled_hover_text(tooltip.to_string()); // response.rs:727, :750
    if ui.is_rect_visible(rect) {
        let fade = theme::hover_fade(ui, resp.id.with("hover"), enabled && resp.hovered(), p.hover);
        let painter = ui.painter();
        painter.rect_filled(rect, theme::radius::ROW, if selected { p.accent_soft } else { fade });
        if selected {
            painter.rect_filled(egui::Rect::from_min_size(
                egui::pos2(rect.left() + 8.0, rect.bottom() - 3.0), egui::vec2(rect.width() - 16.0, 2.0)), 1.0, tint);
        }
        let (g, f) = icon;
        painter.text(egui::pos2(rect.left() + 17.0, rect.center().y), egui::Align2::CENTER_CENTER,
            if has_icons { g } else { f },
            if has_icons { egui::FontId::new(16.0, egui::FontFamily::Name(icons::ICONS.into())) }
            else { egui::FontId::proportional(16.0) },
            theme::glyph_color(tint, dark).gamma_multiply(alpha));
        if let Some(gal) = galley {
            painter.galley(egui::pos2(rect.left() + 34.0, rect.center().y - gal.size().y / 2.0), gal, p.text);
        }
        if resp.has_focus() { theme::focus_ring(painter, rect, &p, theme::radius::ROW); }
    }
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}
```

`bar_fits_labels` (calcul déterministe, une mise en page de galley par commande, mise en cache par egui) :

```rust
fn bar_fits_labels(&self, ui: &egui::Ui, cmds: &[Command]) -> bool {
    let lang = self.lang();
    let font = egui::FontId::proportional(14.0);
    let need: f32 = cmds.iter().map(|c| {
        ui.painter().layout_no_wrap(tr(lang, c.info().label_key).to_string(), font.clone(), Color32::WHITE).size().x + 42.0
    }).sum();
    need + 330.0 <= ui.available_width()   // 330 = retour + pastille + nom + séparateurs + « Quitter l'outil »
}
```

Règles d'état :

```rust
fn command_selected(&self, c: Command) -> bool {
    match c {
        Command::OrganizePages => self.organize.active,
        Command::ToggleComments => self.annotate.comments_open,
        _ => Tool::from_command(c).is_some_and(|t| t == self.annotate.tool),
    }
}
pub fn command_enabled(&self, c: Command) -> bool {
    let grid_sel = !self.organize.selected.is_empty();
    match c {
        Command::Undo => self.can_undo,
        Command::Redo => self.can_redo,
        Command::DuplicateObject | Command::RotateObjectLeft | Command::RotateObjectRight
        | Command::BringToFront | Command::SendToBack | Command::DeleteObject => self.edit.selected.is_some(),
        Command::EditObjectText => self.edit.selected.as_ref()
            .is_some_and(|(_, i)| i.kind == sunipdf_core::ObjectKind::Text),
        Command::CopyText => self.annotate.selection.as_ref().is_some_and(|(_, s)| !s.text.is_empty()),
        // Dans la grille, ces actions portent sur la sélection (même règle que organize.rs:60).
        Command::RotateClockwise | Command::RotateCounterClockwise | Command::DeletePage
        | Command::MovePageLeft | Command::MovePageRight | Command::ExtractPages => !self.organize.active || grid_sel,
        _ => true,
    }
}
```

Largeurs attendues à 1280 px (calcul indicatif, à confirmer par capture) : Commenter ≈ 1160 px avec libellés, donc libellés
affichés ; Protéger et Tampon : libellés ; Modifier (13 commandes) et Organiser (9) : icônes seules, le bouton de l'outil de dessin
sélectionné montrant néanmoins son libellé. Aucune largeur n'est codée en dur par outil.

### 5.6 Palette flottante et barre d'objet

- `annotate::tool_palette` est supprimée (fonction + appel `app.rs:1230`). Les mêmes `Tool` et la même commande `execute`
  alimentent la barre. Le curseur (`editor.rs:page_cursor`), le glissement et `finish_drag` ne changent pas.
- `object_bar` (`editor.rs:609`) garde : trait (couleur + épaisseur), remplissage, « Appliquer à l'objet ». Le bloc
  `for cmd in [RotateObjectLeft … DeleteObject]` (`editor.rs:643-663`) et son `ui.separator()` sont retirés ; `action` et `has_icons`
  deviennent inutiles. Sa position devient `palette_origin` (plus de décalage `+46`, `app.rs:1231`). Il n'est affiché que si
  `annotate.tool.is_edit()` (inchangé), donc uniquement dans l'outil Modifier.

### 5.7 Panneau, barre d'outils principale, menu

**`tools_panel`** (`shell.rs:578`) réécrit :

```rust
pub fn tools_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let (lang, p) = (self.lang(), self.pal());
    /* en-tête : titre `toolbar.all_tools` + bouton ✕ `tools.close` : inchangé */
    ui.separator();
    let active = self.annotate.workspace;
    let mut run = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for ws in Workspace::available() {
            let info = ws.info();
            let resp = hub_row(ui, self.has_icons, info, tr(lang, info.name_key), tr(lang, info.desc_key), active == Some(ws));
            if resp.clicked() {
                resp.surrender_focus(); // sinon le focus reste sur la ligne et bloque les raccourcis à une touche (app.rs:~603)
                run = if active == Some(ws) { Some(Command::HubLeave) } else { ws.activate() };
            }
        }
        ui.add_space(8.0);
        ui.label(RichText::new(tr(lang, "tools.hint")).size(12.0).color(p.text_muted));
    });
    if let Some(c) = run { self.execute(ctx, c); }
}
```

`hub_row` (remplace `tool_row`, `shell.rs:189`) :

```rust
pub fn hub_row(ui: &mut egui::Ui, has_icons: bool, info: &WsInfo, label: &str, desc: &str, active: bool) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let p = theme::palette(dark);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click()); // focusable : sense.rs:60
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(desc.to_string());
    if ui.is_rect_visible(rect) {
        let fade = theme::hover_fade(ui, resp.id.with("hover"), resp.hovered() && !active, p.hover);
        let painter = ui.painter();
        painter.rect_filled(rect, theme::radius::ROW, if active { p.accent_soft } else { fade });
        if active {
            painter.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0),
                egui::vec2(3.0, rect.height() - 16.0)), 2.0, info.color);
        }
        let badge = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 6.0), egui::vec2(28.0, 28.0));
        painter.rect_filled(badge, 7.0, theme::badge_fill(info.color, dark));
        painter.text(badge.center(), egui::Align2::CENTER_CENTER, if has_icons { info.icon.0 } else { info.icon.1 },
            if has_icons { egui::FontId::new(15.0, egui::FontFamily::Name(icons::ICONS.into())) } else { egui::FontId::proportional(15.0) },
            theme::glyph_color(info.color, dark));
        painter.text(egui::pos2(badge.right() + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, label,
            egui::FontId::new(14.0, if active { egui::FontFamily::Name(icons::SEMIBOLD.into()) } else { egui::FontFamily::Proportional }),
            p.text);
        if has_icons && matches!(info.entry, Some(Entry::Mode(_))) {
            painter.text(egui::pos2(rect.right() - 14.0, rect.center().y), egui::Align2::CENTER_CENTER, glyph::CHEVRON_RIGHT,
                egui::FontId::new(12.0, egui::FontFamily::Name(icons::ICONS.into())), p.text_muted);
        }
        if resp.has_focus() { theme::focus_ring(painter, rect, &p, theme::radius::ROW); }
    }
    resp
}
```

**`toolbar`** (`shell.rs:447`) : supprimer la boucle `for tab in ToolTab::ALL` (`shell.rs:~509-535`) et la remplacer par

```
[☰ Menu ▾] │ [Ouvrir] [Enregistrer] [Annuler] [Rétablir] │ [▤ Tous les outils  F9]  ……………  [ⓘ] [🔍 Rechercher outils et commandes  Ctrl+K]
```

- Accès rapide : `quick_button(ui, ctx, Command::OpenDocument | Save | Undo | Redo)` (bouton icône 32 px, `add_enabled`, info-bulle =
  libellé du registre + raccourci, libellé AccessKit par `widget_info`). Activation : `Save` si `document.is_some() && dirty &&
  !saving` (règle actuelle `shell.rs:~544`), `Undo`/`Redo` selon `can_undo`/`can_redo`. Le bouton Enregistrer actuel, à droite, est
  retiré (`shell.rs:~543-551`).
- « Tous les outils » : `Button::new((icône ALL_APPS, libellé `toolbar.all_tools`)).selected(self.show_tools)`, clic =
  `Command::ToggleTools`, info-bulle `menu.view.tools` + `shortcut.tools`.
- Menu « Outils » ajouté après « Fichier » dans le bouton Menu (`shell.rs:~458`) :

```rust
fn tools_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
    let lang = self.lang();
    let mut run = None;
    ui.menu_button(tr(lang, "menu.tools"), |ui| {
        for ws in Workspace::available() {
            let on = self.annotate.workspace == Some(ws);
            if ui.add(egui::Button::new(tr(lang, ws.info().name_key)).selected(on)).clicked() { run = ws.activate(); ui.close(); }
        }
        ui.separator();
        let leave = Command::HubLeave.info();
        if crate::app::menu_item(ui, tr(lang, leave.label_key), tr(lang, "shortcut.hub_leave"), self.annotate.workspace.is_some()) {
            run = Some(Command::HubLeave);
        }
    });
    if let Some(c) = run { self.execute(ctx, c); }
}
```

(`menu_item` d'`app.rs:~1870` passe en `pub(crate)`.)

**`command_color`** (`shell.rs:150`) devient :

```rust
pub fn command_color(command: Command) -> Color32 {
    if let Some(ws) = Workspace::owner_of(command).or_else(|| Workspace::from_command(command)) { return ws.info().color; }
    match command.info().menu { Menu::File => theme::CAT_FILE, _ => theme::CAT_VIEW }   // commandes système
}
```

**`command_icon`** (`shell.rs:246`) : ajouter
`Command::HubEdit | HubOrganize | HubComment | HubProtect | HubStamp => Workspace::from_command(command).map_or((glyph::ALL_APPS, "▤"), |w| w.info().icon),`
et `Command::HubLeave => (glyph::CLOSE, "✕")`.

**Onglet de document actif** (`shell.rs:tab_strip`, polish) : après le `Frame::show` de l'onglet actif, ajouter
`ui.painter().hline(resp.rect.x_range(), resp.rect.top() + 1.0, Stroke::new(2.0, p.accent));` (trait d'accent de 2 px).

### 5.8 Page d'accueil (`shell.rs:635`)

Remplacer le tableau `cards: [(…); 7]` (`shell.rs:~664-720`) et sa boucle par une grille de cartes **de taille fixe** dessinées en
« alloue d'abord, peins ensuite » (la couleur de bordure dépend du survol, connu seulement après l'allocation ; `Frame::show`
imposerait un retard d'une image) :

```rust
const CARD: egui::Vec2 = egui::vec2(244.0, 120.0);
let mut run: Option<Command> = None;
ui.horizontal_wrapped(|ui| {
    ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
    /* carte « Ouvrir un fichier » : fond p.accent_fill, texte blanc, run = OpenDocument (remplace le Frame p.hover actuel) */
    for ws in Workspace::available() {
        let info = ws.info();
        let (rect, resp) = ui.allocate_exact_size(CARD, egui::Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(lang, info.name_key)));
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let t = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered() || resp.has_focus(), 0.12); // context.rs:3214
        let painter = ui.painter();
        if !dark {   // l'ombre noire n'est pas visible en thème sombre : la bordure colorée porte l'effet
            let shadow = egui::epaint::Shadow { offset: [0, 2 + (t * 2.0) as i8], blur: 10 + (t * 6.0) as u8, spread: 0,
                                                color: Color32::from_black_alpha(14 + (t * 14.0) as u8) };
            painter.add(shadow.as_shape(rect, theme::radius::CARD));                                // epaint/shadow.rs:48
        }
        painter.rect(rect, theme::radius::CARD, p.surface,
            Stroke::new(1.0, p.border.lerp_to_gamma(info.color, t)), egui::StrokeKind::Inside);     // ecolor/color32.rs:331
        if resp.has_focus() { theme::focus_ring(painter, rect, &p, theme::radius::CARD); }
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(theme::space::L)), |ui| {     // ui.rs:2194
            ui.horizontal(|ui| { badge(ui, has_icons, info.icon, info.color, 36.0, dark);
                                 ui.label(icons::semibold(tr(lang, info.name_key), 15.0)); });
            ui.add_space(6.0);
            ui.label(RichText::new(tr(lang, info.desc_key)).size(13.0).color(p.text_muted));
        });
        if resp.clicked() { run = ws.activate(); }
    }
});
if let Some(c) = run { self.execute(ctx, c); }   // Direct ou Mode : même chemin
```

`badge` remplace les deux copies du dessin de pastille de `shell.rs` (`tool_row`, `home`) ; `dialog.rs` n'est pas touché. La description est
bornée à deux lignes (cartes de hauteur fixe) : les textes du §7 tiennent en deux lignes à 13 px sur 212 px de large (≈ 34 caractères
par ligne) ; à confirmer par la capture S1.

### 5.9 Outils non implémentés et contrat pour les autres tranches

Règle : **un outil est visible si et seulement si `entry` est `Some`**. `available()` est l'unique filtre ; panneau, accueil, menu et
palette en dérivent. Pour activer un outil, une tranche fait exactement :

1. ajouter ses `Command` (moteur + `REGISTRY`, id stable, `needs_document` correct) ;
2. dans `TABLE`, passer `entry` à `Some(Entry::Direct(cmd))` ou `Some(Entry::Mode(Command::HubXxx))`, renseigner `commands` (chaque
   commande d'**un seul** outil), choisir l'icône ;
3. si Mode : ajouter `HubXxx` au registre (menu `Tools`), le bras `Command::HubXxx` d'`execute` (même ligne que les cinq existants)
   et l'entrée `command_icon` ;
4. rien d'autre : nom, description, couleur et clés i18n existent déjà pour les 14 outils (§7).

Un outil dont les commandes sont créées mais pas encore câblées reste `entry: None` : aucun bouton mort. Le test
`every_registry_command_has_a_home` (§8.2) échoue si une commande n'est ni dans un outil ni dans la liste système : une nouvelle
fonction ne peut pas être oubliée du hub.

### 5.10 Accessibilité et clavier

- **Focus** : `Sense::click()` inclut `FOCUSABLE` (`egui/sense.rs:60-62`) ; Tab/Maj+Tab et flèches déplacent le focus
  (`egui/memory/mod.rs:575-594`) dans l'ordre de création des widgets : barre d'outils → barre d'outil actif → panneau → ...
  Entrée/Espace sur un widget focalisé = clic (`egui/context.rs:1466-1472`). **Aucun code de navigation à écrire** ; les lignes et boutons
  personnalisés sont des `allocate_exact_size(…, Sense::click())`, donc atteignables.
- **Anneau de focus** : `theme::focus_ring` (2 px, `p.focus`) sur `hub_row`, `bar_button`, cartes d'accueil.
- **Focus résiduel** : après un clic ou Entrée sur une ligne ou un bouton, `resp.surrender_focus()` (`egui/response.rs:382`) libère le
  focus : sans cela `handle_input` reste sur sa branche « un widget a le focus » (`app.rs:~603`) et les raccourcis à une touche (H, E...)
  seraient muets.
- **Ouverture au clavier du hub** (optionnel, étape 8) : champ `hub_focus: bool` mis à `true` par `ToggleTools` à l'ouverture ;
  `tools_panel` appelle `resp.request_focus()` (`response.rs:377`) sur la ligne active ou la première, puis remet le champ à `false`.
- **Lecteurs d'écran** : `Response::widget_info` (`response.rs:869`) pose libellé + état sélectionné
  (`fill_accesskit_node_from_widget_info`, `response.rs:926-985` : `set_label`, `set_toggled`). Les libellés réutilisent `hub.*.name` et
  les `label_key` du registre : **aucune clé `a11y.*` nouvelle**.
- **Contraste** : §5.11.2 (mesures).

### 5.11 Polish visuel du chrome (code `theme.rs`)

#### 5.11.1 Jetons

```rust
pub mod space  { pub const XS: f32 = 4.0; pub const S: f32 = 8.0; pub const M: f32 = 12.0; pub const L: f32 = 16.0; pub const XL: f32 = 24.0; }
pub mod radius { pub const CONTROL: u8 = 6; pub const ROW: u8 = 8; pub const CARD: u8 = 12; pub const DIALOG: u8 = 14; }
pub mod font   { pub const CAPTION: f32 = 12.0; pub const BODY: f32 = 14.0; pub const SUBTITLE: f32 = 16.0; pub const TITLE: f32 = 22.0; pub const DISPLAY: f32 = 28.0; }
```
Usage : tout code **nouveau** de la tranche les utilise ; pas de refactor global des littéraux existants (diff minimal).

`Palette` (`theme.rs:24`) gagne trois champs :

| Champ | Clair | Sombre | Rôle |
|---|---|---|---|
| `accent_fill` | `2563EB` | `2563EB` | fond des boutons pleins ; texte blanc ≥ 5,17:1 dans les deux thèmes |
| `accent_soft` | `E9EFFD` | `2C3C59` | ligne/bouton sélectionné (accent à 10 % / 16 % sur la surface) |
| `focus` | `2563EB` | `60A5FA` | anneau de focus clavier |

#### 5.11.2 Défauts de contraste mesurés (WCAG, luminance relative, calcul par script sur ce poste)

| Cas | Avant | Après |
|---|---|---|
| Texte blanc sur bouton plein, **thème sombre** (`dialog::primary` utilise `p.accent` = `60A5FA`, `dialog.rs:~163`) | **2,54:1** | 5,17:1 (`accent_fill`) |
| Texte atténué sur fond (`64748B` / `F8FAFC` ; `94A3B8` / `191E2E`) | 4,55:1 ; 6,47:1 | inchangé (conforme AA) |
| Glyphe de catégorie sur sa pastille, clair (ambre `F59E0B` sur teinte 13 % ; bleu ciel ; vert) | 1,94:1 ; 2,43:1 ; 2,85:1 | ≥ 4,3:1 (règle ci-dessous) |
| Glyphe de catégorie sur sa pastille, sombre (bleu `2563EB` ; violet ; rose sur teinte 32 %) | 2,09:1 ; 2,37:1 ; 2,47:1 | ≥ 3,9:1 |

Règle appliquée à toute couleur d'outil : le glyphe est **assombri de 35 % vers le noir en thème clair, éclairci de 35 % vers le blanc
en thème sombre** ; la pastille garde la teinte brute (13 % / 32 %).

```rust
pub fn glyph_color(base: Color32, dark: bool) -> Color32 {          // lerp_to_gamma : ecolor/color32.rs:331
    base.lerp_to_gamma(if dark { Color32::WHITE } else { Color32::BLACK }, 0.35)
}
pub fn badge_fill(base: Color32, dark: bool) -> Color32 { base.gamma_multiply(if dark { 0.32 } else { 0.13 }) } // :269 (facteur >= 0)
/// Fondu de survol de 100 ms ; `t` dans [0,1] multiplie l'opacité de `color`. Demande seul le repaint (context.rs:3214).
pub fn hover_fade(ui: &egui::Ui, id: egui::Id, hovered: bool, color: Color32) -> Color32 {
    color.gamma_multiply(ui.ctx().animate_bool_with_time(id, hovered, 0.10))
}
pub fn focus_ring(painter: &egui::Painter, rect: egui::Rect, p: &Palette, radius: u8) {
    painter.rect_stroke(rect.shrink(1.0), radius, Stroke::new(2.0, p.focus), egui::StrokeKind::Inside);
}
```

Mesures des 14 couleurs d'outils avec la règle (clair / sombre) : Exporter 5,16 / 4,38 ; Modifier 7,27 / 3,91 ; Créer 5,82 / 4,34 ;
Combiner 7,86 / 3,91 ; Organiser 4,30 / 4,72 ; Commenter 6,92 / 4,11 ; Protéger 6,38 / 4,18 ; Compresser 6,11 / 4,30 ; Tampon 7,30 / 3,90 ;
(masqués) Signer 7,18 / 4,10 ; OCR 4,71 / 4,54 ; Caviarder 9,89 / 3,74 ; Formulaire 5,59 / 4,48 ; Certificat 7,21 / 3,96. Le test
`tool_glyph_contrast` (§8.2) garde le seuil ≥ 3,5:1 pour toute couleur future.

#### 5.11.3 Couleurs d'outils (dans `TABLE`)

Exporter `0EA5E9`, Remplir/Signer `6366F1`, Modifier `E11D48` (= `CAT_EDIT`), Créer `16A34A` (= `CAT_CONVERT`), Combiner `2563EB` (= `CAT_FILE`),
Organiser `F59E0B` (= `CAT_PAGES`), Commenter `8B5CF6` (= `CAT_ANNOTATE`), OCR `06B6D4`, Protéger `0D9488` (= `CAT_SECURITY`), Caviarder `475569`,
Compresser `EA580C`, Formulaire `65A30D`, Tampon `C026D3`, Certificat `DB2777`. Les sept `CAT_*` restent (utilisés ailleurs,
`theme.rs:9-15`) ; seul `command_color` change de source.

#### 5.11.4 Réglages globaux de `theme::apply` (`theme.rs:57-123`)

```rust
v.widgets.active.bg_fill   = p.accent_fill;                 // était p.accent (illisible avec texte blanc en sombre)
v.interact_cursor          = Some(egui::CursorIcon::PointingHand);   // style.rs:1117 ; n'agit que sur egui::Button (widgets/button.rs:385)
for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] { w.expansion = 0.0; } // style.rs:1319 : pas de « pop » au survol
v.popup_shadow  = egui::epaint::Shadow { offset: [0, 4],  blur: 16, spread: 0, color: Color32::from_black_alpha(if dark { 150 } else { 40 }) }; // style.rs:1074
v.window_shadow = egui::epaint::Shadow { offset: [0, 10], blur: 32, spread: 0, color: Color32::from_black_alpha(if dark { 160 } else { 60 }) }; // style.rs:1062
// dans ctx.all_styles_mut :
s.animation_time = 0.10;                                    // style.rs:318
s.interaction.tooltip_delay = 0.35;                         // style.rs:928 (défaut 0,5 s)
s.spacing.scroll = egui::style::ScrollStyle::floating();    // style.rs:465, :643 : barres de défilement flottantes
```
`dialog::primary` (`dialog.rs:~157`) : `.fill(p.accent_fill)`.

#### 5.11.5 Hiérarchie visuelle (avant → après)

| Zone | Avant | Après |
|---|---|---|
| Rangée d'onglets | actif = fond surface + bordure | idem + trait d'accent 2 px en haut de l'onglet actif |
| Barre d'outils | 7 onglets texte + Save/Info/Recherche | accès rapide à icônes, bouton « Tous les outils », recherche (inchangée) ; plus de doublon du panneau |
| Panneau gauche | ~30 lignes plates, 36 px | 9 lignes de 40 px ; actif = `accent_soft` + trait de 3 px ; fondu au survol |
| Barre contextuelle | n/a | 46 px, filet de 2 px de la couleur de l'outil, boutons 34 px, rayon 8 |
| Outils flottants | palette verticale 30 px sur la page | supprimée (dans la barre) ; la page n'est plus masquée |
| Cartes d'accueil | 7 cartes, lien seul cliquable, ombre noire invisible en sombre | 10 cartes 244 × 120 alignées, carte entière cliquable, bordure → couleur de l'outil au survol (fondu 120 ms), ombre seulement en clair |
| Typographie | tailles littérales 11-28 | échelle 12 / 14 / 16 / 22 / 28 pour le code nouveau ; noms d'outils 14 (actif : semi-gras), titres 15-18 semi-gras, descriptions 12-13 `text_muted` |

Thème sombre : on **conserve** la pile de surfaces existante (canvas `0B101A` < chrome `191E2E` < surface `22283A` < hover `2A3648`) et le
texte `F1F5F9` (13,4:1) ; la profondeur passe par les bordures (`343C52`, 1,34:1 sur la surface) et l'éclaircissement de la surface au
survol, jamais par des ombres noires. Aucune de ces valeurs de fond n'est modifiée. Le rendu subjectif (équilibre, densité) n'est pas
mesurable : il sera jugé sur les captures (§8.5) puis par l'utilisateur.

---

## 6. CLI

Aucune sous-commande, aucune option. Justification : le hub est une organisation d'interface ; chaque action ciblée a déjà sa commande
CLI (`merge`, `split`, `extract`, `rotate`, `insert`, `move`, `stamp`, `page-numbers`, `protect`, `unprotect`, `strip-metadata`, `properties`,
`images2pdf`, `export-images`, `optimize`).

Seul effet visible : `sunipdf-cli commands` (JSON) liste 6 entrées de plus, par exemple
`{"id":"hub.edit","menu":"tools","needs_document":true}`, comme pour `view.tools` ou `app.palette`. Aucun test de la CLI ne compte les
entrées aujourd'hui (seul `parses_page_specifications` existe, `main.rs:677`).

---

## 7. i18n

### 7.1 Clés ajoutées (32) : dans `fr-FR.json` **et** `en-US.json`

| Clé | fr-FR | en-US |
|---|---|---|
| `menu.tools` | Outils | Tools |
| `hub.leave` | Quitter l'outil | Leave tool |
| `shortcut.hub_leave` | Échap | Esc |
| `hub.back` | Retour à tous les outils | Back to all tools |
| `hub.export.name` | Exporter un PDF | Export a PDF |
| `hub.export.desc` | Enregistrer les pages en images PNG. | Save the pages as PNG images. |
| `hub.fill_sign.name` | Remplir et signer | Fill & Sign |
| `hub.fill_sign.desc` | Remplir un formulaire et placer une signature. | Fill in a form and place a signature. |
| `hub.edit.name` | Modifier un PDF | Edit a PDF |
| `hub.edit.desc` | Ajouter ou modifier du texte, des images et des formes. | Add or edit text, images and shapes. |
| `hub.create.name` | Créer un PDF | Create a PDF |
| `hub.create.desc` | Assembler des images en un nouveau PDF. | Assemble images into a new PDF. |
| `hub.combine.name` | Combiner des fichiers | Combine files |
| `hub.combine.desc` | Fusionner plusieurs PDF en un seul fichier. | Merge several PDFs into one file. |
| `hub.organize.name` | Organiser les pages | Organize pages |
| `hub.organize.desc` | Pivoter, supprimer, réordonner, diviser et extraire des pages. | Rotate, delete, reorder, split and extract pages. |
| `hub.comment.name` | Commenter | Add comments |
| `hub.comment.desc` | Surligner, souligner, barrer et annoter le texte. | Highlight, underline, strike through and annotate text. |
| `hub.scan_ocr.name` | Numériser et OCR | Scan & OCR |
| `hub.scan_ocr.desc` | Rendre sélectionnable le texte des documents numérisés. | Make the text of scanned documents selectable. |
| `hub.protect.name` | Protéger un PDF | Protect a PDF |
| `hub.protect.desc` | Chiffrer par mot de passe, retirer la protection, effacer les métadonnées. | Encrypt with a password, remove protection, clear metadata. |
| `hub.redact.name` | Caviarder un PDF | Redact a PDF |
| `hub.redact.desc` | Supprimer définitivement des informations sensibles. | Permanently remove sensitive information. |
| `hub.compress.name` | Compresser un PDF | Compress a PDF |
| `hub.compress.desc` | Réduire la taille du fichier sans perte de qualité. | Reduce the file size without quality loss. |
| `hub.prepare_form.name` | Préparer un formulaire | Prepare a form |
| `hub.prepare_form.desc` | Ajouter des champs remplissables à un document. | Add fillable fields to a document. |
| `hub.stamp.name` | Ajouter un tampon | Add a stamp |
| `hub.stamp.desc` | Tampon, filigrane et numérotation des pages. | Stamp, watermark and page numbering. |
| `hub.certificate.name` | Utiliser un certificat | Use a certificate |
| `hub.certificate.desc` | Signer avec un certificat numérique. | Sign with a digital certificate. |

Les 10 clés des cinq outils masqués (descriptions comprises) sont présentes dès maintenant mais **jamais affichées** tant que
`entry == None` ; elles évitent les conflits de catalogue entre tranches.

### 7.2 Clé modifiée

| Clé | fr-FR | en-US |
|---|---|---|
| `tools.hint` | Choisissez un outil ; sans document ouvert, un fichier vous sera demandé. Ctrl+K ouvre la palette de commandes. | Pick a tool; if no document is open, you will be asked for a file. Ctrl+K opens the command palette. |

### 7.3 Clés à supprimer (22), après `Grep` confirmant l'absence d'usage

`toolbar.read`, `toolbar.annotate`, `toolbar.edit`, `toolbar.pages`, `toolbar.security`, `toolbar.convert` ;
`home.card.{read,find,pages,annot,security,convert,edit}.{title,desc}` (14) ; `home.card.use` ; `organize.close`. Vérifié aujourd'hui : usages
uniquement dans `shell.rs` (ToolTab, cartes) et `organize.rs:92`. Le total passe de 380 à 390 clés (380 + 32 − 22). `toolbar.all_tools`
est conservée.

---

## 8. Tests

### 8.1 Pas de test d'intégration `engine.rs`

Aucune API moteur n'est ajoutée : un test dans `crates/sunipdf-core/tests/engine.rs` ne vérifierait rien du hub. Les fonctions ciblées
(rotation, suppression, tampon...) restent couvertes par les 39 tests existants. Aucune fixture nouvelle.

### 8.2 Tests unitaires

`crates/sunipdf-core/src/command.rs` (`cargo test -p sunipdf-core --lib`) : comptage `Menu::Tools` = 6, préfixe `hub.`, unicité des ids
(test existant `ids_are_unique_and_round_trip`, `command.rs:632`).

`apps/sunipdf-desktop/src/workspace.rs` (`cargo test -p sunipdf-desktop`) :

| Test | Assertion |
|---|---|
| `table_order_matches_enum` | `TABLE[i].ws as usize == i` pour 0..14 ; `Workspace::ALL` dans le même ordre |
| `names_and_descriptions_exist_in_both_catalogs` | `tr(lang, key) != key` pour `name_key`/`desc_key` des 14 outils, deux langues |
| `unavailable_tools_are_inert` | `entry == None` ⇒ `commands.is_empty()` ; `available()` n'en contient aucun |
| `available_tools_are_wired` | `Mode(c)` : `c.info().id` commence par `hub.`, `needs_document`, `from_command(c) == Some(ws)`, `commands.len() >= 2` ; `Direct(c)` : `commands == [c]` ; `available().count() >= 9` |
| `every_listed_command_is_registered_and_unique` | chaque commande de chaque outil : `info()` ne panique pas ; aucune commande dans deux outils |
| `every_registry_command_has_a_home` | pour chaque entrée de `REGISTRY` : `owner_of(c).is_some()` **ou** `c` ∈ `SYSTEM_COMMANDS` (liste du §1.1, déclarée dans le test) ; les commandes système ne sont dans aucun outil ; les `Hub*` ne sont dans aucune liste |
| `tool_glyph_contrast` | contraste WCAG de `glyph_color(color, dark)` sur `badge_fill` composé sur `Palette.surface` : ≥ 3,5 pour les 14 couleurs × 2 thèmes (fonction de luminance de 8 lignes dans le test) |
| `switch_resets_drawing_tool` | Commenter + Surligner → `switch(Some(Edit))` : outil = sélection ; `switch(Some(Comment))` conserve Surligner |
| `leaving_organize_closes_grid` | `organize.active = true`, outil Organiser → `switch(None)` ⇒ `organize.active == false` ; Organiser → Commenter aussi |
| `follow_tool_shortcuts` | `follow(ToolHighlight)` sans outil ⇒ `Some(Comment)` ; `follow(ToolRectangle)` depuis Commenter ⇒ `Some(Edit)` ; `follow(ToolSelect)` depuis Modifier ⇒ `None` ; depuis Commenter ⇒ `Some(Comment)` |
| `follow_organize_grid` | `organize.active = true` puis `follow(OrganizePages)` ⇒ `Some(Organize)` ; grille inactive ⇒ inchangé |

`apps/sunipdf-desktop/src/app.rs` : étendre `ui_keys_used_by_app_exist` (`app.rs:1964`) avec
`concat!(include_str!("app.rs"), include_str!("main.rs"), include_str!("shell.rs"), include_str!("workspace.rs"))` pour vérifier aussi les clés
littérales des nouveaux fichiers (seuil `checked > 40` inchangé). Si un faux positif apparaît (chaîne en minuscules avec un point qui n'est pas
une clé), l'exclure dans le filtre plutôt que d'abandonner l'extension. Les tests `registry_keys_exist_in_both_catalogs` (`app.rs:~1995`) et
`sunipdf-i18n` (parité, valeurs non vides, placeholders) couvrent déjà les `Hub*` et les nouvelles clés.

### 8.3 Test headless de mise en page et d'accessibilité (sans fenêtre)

`Context::run_ui(RawInput, FnMut(&mut Ui)) -> FullOutput` existe (`egui/context.rs:794`) ; `Context::enable_accesskit` (`context.rs:3701`) ;
`PlatformOutput.accesskit_update: Option<TreeUpdate>` (`egui/data/output.rs:176`) ; `egui::accesskit` est ré-exporté (`egui/lib.rs:434`), nœuds
`(NodeId, Node)` avec `Node::label()` (`accesskit-0.24.1/src/lib.rs:1916`, `TreeUpdate` `:2829`). Test `hub_rows_expose_labels_and_fit` dans
`shell.rs` :

1. `ctx = egui::Context::default(); ctx.enable_accesskit();` écran 1280 × 780 dans `RawInput::screen_rect`.
2. Dans `run_ui`, dessiner une `hub_row` par outil disponible (palette claire puis sombre) et un `bar_button` par commande de Commenter.
3. Assertions : hauteur de chaque `hub_row` = 40, largeur = largeur disponible ; `FullOutput.shapes` non vide ; le `TreeUpdate` contient un
   nœud dont `label()` vaut `tr(fr, "hub.comment.name")` marqué sélectionné/non sélectionné, et un nœud par bouton de barre dont le libellé n'est pas
   un caractère de la zone privée `U+E000..U+F8FF`.

Limite assumée : ne vérifie ni les couleurs affichées ni l'alignement fin. Si l'API AccessKit se révèle trop lourde à manipuler, conserver
seulement les assertions de géométrie et l'absence de panique, et le noter dans `KNOWN_ISSUES.md`.

### 8.4 Audit des littéraux (critère « aucune chaîne UI littérale »)

Avec l'outil `Grep` (motif multi-alternatives, répertoire `apps/sunipdf-desktop/src`) :

```
RichText::new\("[^"]*[A-Za-zÀ-ÿ]{3}|Button::new\("[^"]*[A-Za-zÀ-ÿ]{3}|on_hover_text\("[^"]*[A-Za-zÀ-ÿ]{3}|\.label\("[^"]*[A-Za-zÀ-ÿ]{3}|\.heading\("
```

Attendu : aucune ligne ajoutée par la tranche (les symboles de repli `"✕"`, `"+"`, `"⚠ {}"` et `"SuniPDF"` existants ne contiennent pas trois lettres
consécutives ou sont déjà documentés). Résultat à consigner dans le rapport final. Puis `.\scripts\check-i18n.ps1` (parité) et
`cargo test -p sunipdf-i18n -p sunipdf-desktop`.

### 8.5 Scénarios de capture (`scripts/dev-screenshot.ps1`)

Rappel : un seul lancement à la fois ; coordonnées relatives à la fenêtre ; préférences dans `%APPDATA%\SuniPDF\settings.json` à sauvegarder
avant et restaurer après. Fixture : `crates\sunipdf-core\tests\fixtures\two-pages.pdf` (page 1 : 612 × 792 pt, texte « SuniPDF page 1 » à
(72, 700) pt, corps 24). Nouvelle disposition : avec une barre contextuelle, la page commence à y ≈ 186 px (au lieu de ≈ 140) et x ≈ 440 (panneau
d'outils et miniatures ouverts) ; **ajuster les glissements après une première capture sans `-Drag`**.

| # | Commande | À vérifier |
|---|---|---|
| S1 | `.\scripts\dev-screenshot.ps1 -Out s1-home.png` | carte « Ouvrir un fichier » + 9 cartes (Exporter, Modifier, Créer, Combiner, Organiser, Commenter, Protéger, Compresser, Tampon), **aucune** carte Remplir/OCR/Caviarder/Formulaire/Certificat ; panneau de gauche à 9 lignes ; pastilles lisibles |
| S2 | `-Pdf crates\sunipdf-core\tests\fixtures\two-pages.pdf -Out s2-doc.png` | pas de barre contextuelle ; accès rapide Ouvrir/Enregistrer/Annuler/Rétablir (Annuler/Rétablir grisés) ; bouton « Tous les outils » enfoncé |
| S3 | `-Pdf …two-pages.pdf -Commands 'hub.comment' -Out s3-comment.png` | barre violette de 46 px, 7 boutons **avec libellés** à 1280 px, « Sélectionner du texte » sélectionné, « Copier » grisé, ligne « Commenter » active dans le panneau, **plus de palette flottante** |
| S4 | `-Commands 'hub.comment,tool.highlight' -Drag '480,290,700,290' -Out s4-highlight.png` | surlignage jaune posé, bouton « Surligner » sélectionné |
| S5 | `-Commands 'hub.edit,tool.rectangle' -Drag '470,300,620,380' -Out s5-edit.png` | barre rose, boutons en icônes seules (13 commandes), Rectangle sélectionné avec son libellé, objet sélectionné ⇒ actions activées, barre d'objet réduite (trait, remplissage, Appliquer) |
| S6 | `-Commands 'hub.organize' -Out s6-organize.png` | grille ouverte, barre ambre à 9 actions, rotation/suppression grisées sans sélection, barre interne réduite au compteur et « Tout sélectionner » |
| S7 | `-Commands 'hub.protect' -Out s7-protect.png` ; puis `-KeysAfter '{ESC}'` | 4 boutons avec libellés ; après Échap, barre disparue (retour à la lecture) |
| S8 | `-Commands 'hub.stamp'` | 2 boutons magenta ; un clic sur Tampon ouvre le dialogue `dialog::show` |
| S9 | `-Commands 'hub.comment,edit.find' -KeysAfter '{ESC}'` | 1er Échap ferme la recherche **sans** quitter l'outil ; 2e Échap quitte. Vérifier aussi : dialogue Propriétés ouvert, Échap ne quitte pas l'outil ; Échap dans le champ « numéro de page » du rail ne quitte pas l'outil (garde `focus_last_frame`) |
| S10 | `-Commands 'hub.comment' -KeysAfter '{TAB}{TAB}{TAB}{DOWN}'` | anneau de focus visible sur un bouton de barre ou une ligne du panneau, déplacement aux flèches ; taper `h` juste après Entrée sur une ligne doit encore sélectionner Surligner |
| S11 | `-Pdf "a.pdf;b.pdf" -Commands 'hub.comment' -KeysAfter '^{TAB}'` | l'onglet 2 (outil Commenter) → onglet 1 sans barre ; Ctrl+Tab suivant restitue la barre de l'onglet 2 |
| S12 | S1, S3, S5 avec `"dark_theme": true` dans `settings.json` | lisibilité thème sombre : texte, glyphes (règle +35 % blanc), bouton plein des dialogues (5,17:1), absence d'ombres noires |
| S13 | S1 et S3 avec `"language": "en-US"` | libellés anglais, aucune clé brute `hub.*` affichée, barre toujours sur une ligne |
| S14 | S3 et S5 avec fenêtre redimensionnée à 720 px de large | libellés masqués, aucune coupure, bouton « Quitter l'outil » visible |

L'agent qui implémente doit **regarder** chaque capture avant de la déclarer bonne ; la validation finale reste à l'utilisateur.

---

## 9. Nouvelles dépendances

**Aucune.** Tout repose sur `egui`/`eframe` 0.36.2 (déjà dans `Cargo.lock`) et `sunipdf-core`. `egui::accesskit` est ré-exporté par `egui`
(`egui/lib.rs:434`) : pas besoin d'ajouter `accesskit` au `Cargo.toml` du desktop.

---

## 10. Risques, ordre d'implémentation, estimation

### 10.1 Risques et pièges connus

| Risque | Parade |
|---|---|
| Conflits de fusion : les autres tranches modifient aussi `command.rs`, `app.rs`, `shell.rs` (commandes et outils) | Faire **atterrir le hub en premier** (prérequis aucun) ; les autres tranches ajoutent leurs commandes dans `TABLE` (§5.9) au lieu de `ToolTab::commands`. Commits courts, un par étape. La tranche « vue continue » modifie `document_area` (`app.rs:1058`) : rebaser autour des deux lignes `tool_palette`/`object_bar` (`app.rs:1230-1231`). |
| Régression perçue : la palette flottante disparaît | Raccourcis V/H/U/X/N/E/T/I/R/O/L inchangés et ouvrent l'outil ; point à valider avec l'utilisateur (§10.4). |
| Échap surchargé (recherche, dialogues, sélection d'objet, champs, outil) | Garde `esc_leaves_workspace` (+ `focus_last_frame`) et arrêts supplémentaires de `handle_input` ; scénario S9. Un seul Échap = un seul effet. |
| Focus résiduel : les raccourcis à une touche deviennent muets (`app.rs:~603`) | `surrender_focus()` après activation ; scénario S10 (taper H juste après Entrée sur une ligne). |
| Les glyphes de repli (sans Segoe Fluent Icons) des 5 nouvelles icônes ne sont pas vérifiés | Documenter dans `KNOWN_ISSUES.md` A1 ; non bloquant. |
| Les points de code `E72B` (retour) et `EDE1` (export) existent dans la police mais leur dessin est supposé d'après la table MDL2 | Contrôle visuel S2/S3 ; sinon remplacer par `E76B` (chevron gauche) / `E898`. |
| Largeur de la barre sous 800 px : elle n'est pas défilante | Libellés masqués dynamiquement (§5.5) ; S14. Modifier = 13 boutons de 34 px + 1 séparateur + nom + « Quitter » ≈ 700 px, soit la largeur minimale de fenêtre (720 px) : **à mesurer** ; sinon regrouper les actions d'objet dans un menu `…`. |
| Une commande ajoutée sans propriétaire casse le test de couverture | C'est voulu (§5.9, §8.2). |
| Numéro de décision `D021` possiblement pris par une autre tranche | Prendre le premier numéro libre au moment du merge. |
| Le test AccessKit peut exiger des API non triviales | Dégradation prévue (§8.3). |

### 10.2 Étapes compilables (un commit chacune, `cargo build -p sunipdf-desktop -j 2` après chaque étape)

1. **Registre** : `command.rs` (§3.2) + bras `Hub*` provisoires dans `execute` (`{}` avec commentaire « câblé à l'étape 3 ») et `command_icon` ;
   clés `hub.*.name`, `hub.leave`, `shortcut.hub_leave`, `menu.tools` dans les deux catalogues.
   `cargo test -p sunipdf-core -p sunipdf-i18n -p sunipdf-desktop -j 2`.
2. **Données** : `theme.rs` (jetons, champs de palette, 4 helpers, réglages `apply`), `icons.rs` (5 glyphes : `EXPORT EDE1`, `EDIT_DOC E70F`,
   `ADD E710`, `CHEVRON_RIGHT E76C`, `ARROW_LEFT E72B`), `workspace.rs` (table, fonctions, `switch`, `follow`, tests §8.2), clés `hub.*.desc` et
   `hub.back`, `command_color` sur la table. `#![allow(dead_code)]` temporaire en tête de `workspace.rs`, retiré à l'étape 5. Tests unitaires verts.
3. **État** : `AnnotateState.workspace`, `Tool::from_command`, `enter/leave/esc_leaves`, `focus_last_frame`, bras `Hub*` réels, `follow` dans les bras
   d'outils et d'`OrganizePages`, Échap dans `handle_input`, arrêts supplémentaires. Pas encore d'UI : vérifier avec
   `SUNIPDF_STARTUP_COMMANDS=hub.comment` et un `log::debug!` de l'outil actif.
4. **Barre contextuelle** : `workspace_bar`, `bar_button`, `bar_fits_labels`, `command_enabled/selected`, helpers de `shell.rs`, `Panel::top("workspace")`,
   suppression de `tool_palette`, retrait des 6 boutons de `object_bar`, réduction d'`organize_toolbar`. Captures S3-S8.
5. **Panneau, barre d'outils, menu** : `hub_row`, `tools_panel`, `toolbar` (accès rapide, bouton « Tous les outils », menu Outils), suppression de
   `ToolTab`/`tool_tab`/`tool_row`, modification de `tools.hint`. Captures S2, S10, S11.
6. **Accueil** : cartes fixes cliquables, carte « Ouvrir un fichier » pleine. Captures S1, S13.
7. **Polish** : onglet actif, `dialog::primary` en `accent_fill`, passes sombre (S12) et étroite (S14), ajustements de couleurs/espacements après
   lecture des captures.
8. **Clavier, AccessKit, audit** : `hub_focus` optionnel, test headless §8.3, extension du test de clés, `Grep` des littéraux, suppression des 22 clés
   obsolètes (après Grep), `check-i18n.ps1`.
9. **Garde-fous complets** puis documentation (§11) : `cargo test --workspace -j 2`, `cargo clippy --workspace --all-targets -j 2 -- -D warnings`,
   `cargo fmt --all -- --check`, `.\scripts\check-i18n.ps1`.

### 10.3 Estimation

**M** (≈ 1 000 lignes touchées sur 10 fichiers, 0 dépendance, 0 moteur). Le polish de l'étape 7 est la partie la moins prévisible (boucles de
capture) : si la validation visuelle s'éternise, découper en deux livraisons (étapes 1-6, puis 7-9).

### 10.4 Questions ouvertes (utilisateur / orchestrateur)

1. Valider « une commande = un seul outil » : `Watermark` et `PageNumbers` ne sont plus sous « Modifier » (ils sont sous « Tampon »), `InsertPages` est
   sous « Organiser » et pas sous « Combiner ».
2. Accepter la suppression de la palette flottante (au profit de la barre contextuelle) ?
3. Accepter que Exporter / Créer / Combiner / Compresser n'aient pas de barre (ouverture directe de la boîte de dialogue) ?
4. Ordre des outils : ordre PDFCraft (retenu) ou par fréquence d'usage ?
5. Faut-il une section « Bientôt » grisée listant les 5 outils absents ? Le plan dit **non** (règle « pas de bouton factice »).

---

## 11. Documentation à mettre à jour

| Fichier | Contenu |
|---|---|
| `docs/PROJECT_STATUS.md` | nouvelle ligne `SUNI-040 Hub d'outils` (cocher seulement avec résultats réels), tableau « Dernière exécution » (tests, clippy, fmt, check-i18n, captures inspectées S1-S14), journal ; commandes au registre : 63 → 69 |
| `docs/NEXT_TASKS.md` | contrat §5.9 pour les tranches Remplir/Signer, OCR, Caviarder, Formulaire, Certificat ; retirer toute mention de `ToolTab` |
| `docs/DECISIONS.md` | `D021` (ou premier numéro libre) : hub par table de données, Direct/Mode, un propriétaire par commande, suppression de la palette flottante, règles de contraste des glyphes ; amendement de D020 (« outils de catégorie colorés » → couleur d'outil) |
| `docs/FEATURE_MATRIX.md` | ligne UI « Hub d'outils par tâches » : **Fait** pour 9 outils sur 14 ; les 5 autres restent « Planifié » |
| `docs/KNOWN_ISSUES.md` | pas de test de rendu au pixel ; glyphes de repli non vérifiés (A1) ; outils Direct sans barre ; focus résiduel et Échap (comportement) ; barre non défilante |
| `docs/ARCHITECTURE.md` | arborescence : `apps/sunipdf-desktop/src/workspace.rs`, flux « outil → commandes » |
| `docs/ENGINEERING_NOTES.md` | §5 captures : page à y ≈ 186 avec barre (140 sans) ; pièges egui rencontrés (allocation avant peinture pour le survol, `surrender_focus`, `interact_cursor` limité aux `Button`, Échap efface le focus avant la lecture de la touche) |
| `docs/I18N.md` | espace de noms `hub.*` et règle « les clés des outils masqués existent déjà » |
| `docs/USER_GUIDE.fr.md`, `docs/USER_GUIDE.en.md` | section « Tous les outils » : liste des 9 outils, quitter un outil (Échap), raccourcis conservés ; les autres outils arriveront avec leur fonctionnalité |
| `docs/ROADMAP.md`, `CHANGELOG.md` (section « Non publié ») | entrée du hub ; rien n'est annoncé pour les 5 outils absents |
| `CLAUDE.md` (liste « Modules existants ») | ajouter `workspace.rs` ; garder la règle « pas de bouton factice » |
| `scripts/dev-screenshot.ps1` | exemple d'en-tête : `-Commands "hub.comment,tool.highlight"` |

---

## Annexe A : index des citations (API relues dans les sources)

| API utilisée | Source |
|---|---|
| `Sense::click()` = `CLICK \| FOCUSABLE` | `egui-0.36.2/src/sense.rs:60-62` |
| Entrée/Espace = clic sur widget focalisé | `egui/context.rs:1466-1472` |
| Flèches/Tab → direction de focus ; Échap efface le focus (début de passe) | `egui/memory/mod.rs:575-598` |
| `Response::request_focus` / `surrender_focus` | `egui/response.rs:377`, `:382` |
| `Response::widget_info` + `fill_accesskit_node_from_widget_info` (`set_label`, `set_toggled`) | `egui/response.rs:869-885`, `:926-985` |
| `WidgetInfo::labeled` / `selected` ; `WidgetType::{Button, SelectableLabel, Panel}` | `egui/data/output.rs:658`, `:668` ; `egui/lib.rs:623-668` |
| `Response::on_hover_text`, `on_disabled_hover_text`, `on_hover_cursor` | `egui/response.rs:727`, `:750`, `:762` |
| `Context::animate_bool_with_time(id, bool, f32) -> f32` | `egui/context.rs:3214` |
| `Panel::top`, `exact_size(f32)`, `frame`, `show` | `egui/containers/panel.rs:265`, `:405`, `:413`, `:422` |
| `UiBuilder::max_rect` / `sense` ; `Ui::scope_builder` | `egui/ui_builder.rs:103`, `:168` ; `egui/ui.rs:2194` |
| `Painter::layout_no_wrap`, `galley`, `rect`, `rect_filled`, `rect_stroke`, `text`, `hline` | `egui/painter.rs:503`, `:529`, `:380`, `:397`, `:406`, `:469`, `:332` |
| `Galley::size()` | `epaint-0.36.2/src/text/text_layout_types.rs:1017` |
| `Shadow { offset: [i8;2], blur: u8, spread: u8, color }` ; `Shadow::as_shape(rect, radius)` | `epaint/src/shadow.rs:10-27`, `:48` |
| `Color32::gamma_multiply`, `lerp_to_gamma` | `ecolor-0.36.2/src/color32.rs:269`, `:331` |
| `Visuals::interact_cursor` ; appliqué par `Button` | `egui/style.rs:1117` ; `egui/widgets/button.rs:385` |
| `Visuals::popup_shadow`, `window_shadow`, `WidgetVisuals::expansion` | `egui/style.rs:1074`, `:1062`, `:1319` |
| `Style::animation_time`, `Interaction::tooltip_delay`, `Spacing::scroll`, `ScrollStyle::floating()` | `egui/style.rs:318`, `:928`, `:465`, `:643` |
| `Button::selected`, `min_size`, `fill`, `corner_radius`, `frame` | `egui/widgets/button.rs:270`, `:193`, `:143`, `:200`, `:166` |
| `Context::run_ui`, `enable_accesskit` ; `PlatformOutput::accesskit_update` ; `egui::accesskit` | `egui/context.rs:794`, `:3701` ; `egui/data/output.rs:176` ; `egui/lib.rs:434` |
| `Node::label()` ; `TreeUpdate` | `accesskit-0.24.1/src/lib.rs:1916`, `:2829` |
| Registre : `Command`, `Menu`, macro `cmd!`, `REGISTRY`, tests | `crates/sunipdf-core/src/command.rs:5`, `:72`, `:95`, `:109`, `:632-649` |
| Points d'intégration desktop | `app.rs:591` (arrêts), `:733` (F9), `:793` (`execute`), `:923-941` (outils), `:986` (grille), `:1097` et `:1230-1231` (palette/objet), `:1761-1800` (panneaux) ; `shell.rs:150,189,246,447,578,635` ; `annotate.rs:88,108` ; `editor.rs:609,643` ; `organize.rs:36` ; `tabs.rs:69,99` ; `dialog.rs` (`primary`) |
| Présence des glyphes `EDE1, E70F, E710, E76C, E72B` dans `SegoeIcons.ttf` | PowerShell `GlyphTypeface.CharacterToGlyphMap` sur ce poste (présence seulement, pas l'apparence) |
