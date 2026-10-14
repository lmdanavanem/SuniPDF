# Notes d'ingénierie (à lire avant de modifier le code)

Ce document rassemble ce qu'il faut savoir pour livrer une tranche verticale sans casser le dépôt :
commandes, garde-fous, patrons de code et pièges déjà rencontrés. Il complète `CLAUDE.md` et
`docs/ARCHITECTURE.md`.

## 1. Environnement et commandes

- Poste de référence : Windows 11, 14 Go de RAM. **Toujours `-j 2`** avec cargo, **jamais deux cargo en
  parallèle**, jamais `cargo clean` (une reconstruction complète coûte plus de dix minutes).
- `cargo` n'est pas dans le PATH par défaut :
  - Git Bash : `export PATH="$HOME/.cargo/bin:$PATH"`
  - PowerShell : `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`
- Windows PowerShell 5.1 transforme la sortie d'erreur de cargo en exception : lancer les scripts de
  packaging depuis Git Bash (`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/x.ps1`).
- PDFium : `third_party/pdfium/win-x64/bin/pdfium.dll` (ignoré par git, `scripts/fetch-pdfium.ps1`).

### Garde-fous obligatoires avant tout commit (les quatre doivent être verts)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -j 2 -- -D warnings
cargo test --workspace -j 2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-i18n.ps1
```

`cargo fmt --all` reformate : relire un fichier (ou `grep -n`) avant de l'éditer après un formatage.

## 2. Écrire du code sans se piéger

- Modifier les fichiers avec les outils d'édition, pas avec des heredocs shell : les heredocs mangent
  les `\n`, `\u{...}` et les apostrophes. Pour un script de modification, l'écrire dans un fichier puis
  l'exécuter avec `python -I fichier.py`.
- Aucune chaîne visible par l'utilisateur en dur dans `apps/` : `tr(lang, "cle")` ou
  `tr_args(lang, "cle", &[("nom", valeur)])`. Toute clé va dans `assets/i18n/fr-FR.json` **et**
  `assets/i18n/en-US.json` (mêmes clés, JSON indenté de 2 espaces, UTF-8 sans BOM).
- La CLI parle anglais et sort du JSON (`serde_json::json!`), erreurs via `EngineError`.
- L'UI n'importe jamais `pdfium-render` : tout passe par `sunipdf-core` et par le thread moteur.

## 3. Patrons de cette base de code

### Ajouter une opération d'édition annulable
1. Moteur : méthode `impl Document<'_>` dans le module concerné de `crates/sunipdf-core/src/`
   (`edit.rs`, `annot.rs`, `stamp.rs`, `pages.rs`…), erreurs `EngineError`, export dans `lib.rs`.
2. Thread moteur (`apps/sunipdf-desktop/src/worker.rs`) : variante dans `EditOp`, bras dans
   `fn apply(doc, op)`. `State::edit` prend un instantané d'octets avant, l'empile dans l'annulation et
   renvoie `Response::DocumentChanged` ; en cas d'erreur l'instantané est restauré.
3. Lecture seule : variante `Request` + `Response` (voir `ObjectAt` / `ObjectSelected`).
4. Tâche de fichier longue (sortie dans un autre fichier) : `BatchOp` + `BatchResult`
   (voir `tools.rs`, `on_batch_done`).

### Ajouter une commande
1. `crates/sunipdf-core/src/command.rs` : variante dans `enum Command`, entrée `cmd!(Variante, "menu.id",
   "cle.libelle", Some("cle.raccourci") | None, a_besoin_d_un_document, Menu)`.
2. `apps/sunipdf-desktop/src/shell.rs` : bras dans `command_icon` (glyphe de `icons.rs` + repli texte).
   `apps/sunipdf-desktop/src/workspace.rs` : placer la commande dans le groupe de **son outil** (`TABLE`,
   un seul outil par commande, couleur et barre en découlent) ou, pour une commande système, dans
   `SYSTEM_COMMANDS` du test `every_registry_command_has_a_home`. Contrat complet : `docs/NEXT_TASKS.md`.
3. `apps/sunipdf-desktop/src/app.rs` : bras dans `execute`, raccourci dans `handle_input`
   (`key(Modifiers::…, Key::…, Command::…)`). Les raccourcis à une lettre sont ignorés quand un champ de
   saisie ou une boîte de dialogue a le clavier.
4. Clés i18n du libellé et du raccourci dans les deux catalogues.
Une commande qui exige un document et qui est lancée sans document ouvre d'abord un fichier puis
s'exécute (`after_open`).

### Ajouter une boîte de dialogue
Toujours `crate::dialog::show(ctx, dialog::Spec::new(id, titre, dark, has_icons).subtitle(..).icon(..)
.width(..), |ui| corps, |ui| pied)` avec `dialog::primary`, `dialog::secondary`, `dialog::danger`
(le pied est aligné à droite : le premier bouton ajouté est l'action principale). Pas d'`egui::Modal`
direct. Donner le focus au premier champ une seule fois
(`if ui.memory(|m| m.focused().is_none()) { champ.request_focus(); }`), sinon les autres champs
deviennent inéditables. Ajouter l'état « ouvert » à la garde en tête de `handle_input` pour bloquer les
raccourcis pendant la saisie.

### État par onglet
L'état d'interface lié au document (page, zoom, sélection, outils) est échangé par `tabs.rs`
(`take_tab` / `put_tab`). Tout nouvel état lié au document doit y être ajouté, sinon il fuit d'un
onglet à l'autre.

### Coordonnées
L'API du moteur parle en **espace d'affichage** : points, origine en bas à gauche de la page telle
qu'affichée (après `/Rotate`, boîte MediaBox ∩ CropBox) ; l'écran a l'origine en haut à gauche.
Conversions UI : `annotate::to_pdf(pos, page_rect, zoom, page_h_pt)` et, dans l'autre sens,
`page_rect.min + vec2(x * k, (page_h_pt - y) * k)` avec `k = effective_zoom * SCREEN_PER_PDF_POINT`.
Dans le moteur, toute nouvelle fonction qui touche des objets, du texte ou des annotations convertit avec
`geometry::geometry_of(&page)` (`to_user`, `rect_to_user`, `vec_to_user`, `upright_matrix`, et l'inverse en
sortie) ; tester sur `tests/fixtures/boxes.pdf` (pages pivotées 90/180/270 et rognées).

## 4. Pièges des bibliothèques

### pdfium-render 0.9.4 / PDFium 7881
- PDFium n'est pas thread-safe : un seul thread moteur. Il ne se réinitialise pas dans un même
  processus : les tests d'intégration partagent un moteur unique (`engine()` dans `tests/engine.rs`),
  jamais `Engine::new()` directement dans un test.
- Un objet de page modifié en place (texte, matrice, couleur) n'est écrit qu'après
  `page.regenerate_content()` ; l'ajout et la suppression régénèrent seuls.
- `page.objects()` emprunte la page ; pour garder un objet tout en passant `&mut page` à une autre
  fonction, l'obtenir par `page.objects_mut().get(i)`.
- L'ordre de dessin suit les flux de contenu : réordonner des objets n'a d'effet qu'après fusion des
  flux (`merge_content_streams`, lopdf).
- Les limites d'un objet texte partent de la ligne de base (les jambages dépassent en dessous).
- PDFium ne chiffre pas à l'écriture et ne modifie pas les signets : ces opérations passent par lopdf
  sur les octets du document (`security.rs`), puis `Document::restore`.

### egui / eframe 0.36
- `impl eframe::App { fn ui(&mut self, ui, frame) }`, panneaux `egui::Panel::top/left/right/bottom`,
  `Frame::new().corner_radius(..)`, `ComboBox::from_id_salt`, `StrokeKind`, `EllipseShape { angle, .. }`.
- Icônes : `icons::icon(has_icons, glyph::X, "repli", taille)` ; ne jamais concaténer un glyphe d'icône
  dans une chaîne de texte (police différente) : utiliser `Button::new((icone, libelle))` ou peindre
  avec `painter.text(.., FontId::new(taille, FontFamily::Name(icons::ICONS.into())), ..)`.
- Le fond « inactif » du thème est la couleur de surface : un `Slider` a besoin d'un
  `ui.visuals_mut().widgets.inactive.bg_fill` local pour que son rail soit visible.
- Clavier : `ctx.input_mut(|i| i.consume_key(Modifiers::…, Key::…))` pour éviter qu'une touche serve
  deux fois dans la même image.
- Origine d'un glissement : `ui.input(|i| i.pointer.press_origin())`, pas la position au moment où le
  seuil de glissement est franchi.
- Widget dessiné à la main : `allocate_exact_size(.., Sense::click())` (focalisable), puis
  `resp.widget_info(..)` pour un libellé AccessKit lisible (jamais le glyphe d'icône), peinture après
  l'allocation (le survol n'est connu qu'ensuite). Après une activation, `resp.surrender_focus()` : un widget
  focalisé fait taire les raccourcis à une lettre de `handle_input`.
- egui retire le focus sur Échap **avant** que l'application lise la touche : `SuniApp::focus_last_frame`
  garde l'état de l'image précédente (Échap dans un champ ne quitte pas l'outil actif).
- Échap passe par un seul point : `workspace::take_escape` (ordre dans `EscState::action`, testé). Les menus
  et listes egui (`menu_button`, `ComboBox`, sélecteur de couleur) ne lisent Échap qu'au moment de leur
  dessin : `handle_input`, qui tourne avant, consommerait la touche. `egui::Popup::is_any_open(ctx)` lu en
  début d'image donne l'état de l'image précédente ; `take_escape` ferme alors le menu lui-même
  (`Popup::close_all`) et consomme la touche. Toute nouvelle action sur Échap s'ajoute à `EscState`.
- `Visuals::interact_cursor` ne s'applique qu'aux `egui::Button` ; les widgets maison posent
  `on_hover_cursor(PointingHand)`. `ui.set_max_width(w)` peut **élargir** : borner par `available_width()`.
- Une rangée `ui.horizontal` démarre à `interact_size.y` de haut ; pour centrer des boutons plus hauts
  sur une pastille voisine, régler `interact_size.y` à leur hauteur.
- Test sans fenêtre : `Context::run_ui` + `enable_accesskit()` ; vider `out.textures_delta` avant de
  jeter la sortie (assertion de débogage d'epaint).
- Ctrl+C / X / V n'arrivent **jamais** comme `Event::Key` : egui-winit les change en `Event::Copy`, `Cut`,
  `Paste(texte)` (Paste seulement si le presse-papiers système contient du texte). Les lire dans `i.events`.
- Une fenêtre surgissante (`ComboBox`, `Area`) garde la taille de l'image précédente : un contenu qui grandit
  après coup (liste chargée en différé) est tronqué. Donner une hauteur fixe à la partie variable
  (`ScrollArea::max_height(..).auto_shrink([false, false])`).
- Un `ui.horizontal` imbriqué dans `horizontal_wrapped` ne passe pas à la ligne : il déborde. Préférer des
  widgets simples, ou une rangée par `horizontal_wrapped`.

## 5. Vérification visuelle

`scripts/dev-screenshot.ps1 -Out capture.png [-Pdf "a.pdf;b.pdf"] [-Commands "id1,id2"]
[-Drag "x1,y1,x2,y2;x1,y1,x2,y2"] [-Keys "^k"] [-KeysAfter "{F2}|^c|^v"] [-Delay 5]` lance `target\debug\sunipdf.exe`,
exécute les commandes au démarrage (`SUNIPDF_STARTUP_COMMANDS`), simule des glissements (coordonnées
relatives à la fenêtre, page affichée à partir de x≈440, y≈140 avec le panneau d'outils ouvert, y≈190 quand
la barre d'un outil est affichée ; un glissement de hauteur nulle ne sélectionne pas de texte), `-Width`/`-Height`
imposent la taille de la fenêtre, capture
la fenêtre puis ferme l'application. Un seul lancement à la fois : le script prend le premier plan et
déplace la souris. Journal : `$env:SUNIPDF_LOG = "fichier.log"` et `$env:RUST_LOG = "debug"`.
Les préférences sont dans `%APPDATA%\SuniPDF\settings.json` : restaurer le fichier si un test le modifie.

## 6. Définition de « terminé » pour une tranche

Code réel de bout en bout (moteur, thread moteur, interface, CLI si pertinent), commande accessible
depuis le panneau d'outils et la palette, clés i18n fr et en, test d'intégration qui vérifie le résultat
(relecture du texte, pixels du rendu, fichier rouvert), cas d'erreur gérés, capture d'écran regardée,
documentation à jour (`PROJECT_STATUS.md`, `FEATURE_MATRIX.md`, `CHANGELOG.md` section « Non publié »,
guides utilisateur fr et en, `KNOWN_ISSUES.md` pour les limites), les quatre garde-fous verts.
Ce qui n'est pas fait reste dans la roadmap, avec la raison.
